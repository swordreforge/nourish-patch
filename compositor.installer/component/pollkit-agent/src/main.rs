use iced::widget::{button, column, row, text, text_input};
use iced::{Alignment, Element, Task};
use std::collections::HashMap;
use std::error::Error;
use zbus::{connection, interface, proxy};

// ---- D-Bus Proxy Definitions ----

#[proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority"
)]
trait Authority {
    fn register_authentication_agent(
        &self,
        subject: &Subject,
        locale: &str,
        object_path: &str,
    ) -> zbus::Result<()>;

    fn unregister_authentication_agent(
        &self,
        subject: &Subject,
        object_path: &str,
    ) -> zbus::Result<()>;
}

#[derive(serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
struct Subject {
    subject_kind: String,
    subject_details: HashMap<String, zbus::zvariant::OwnedValue>,
}

// ---- Background D-Bus Daemon ----

struct PolkitAgent;

#[interface(name = "org.freedesktop.PolicyKit1.AuthenticationAgent")]
impl PolkitAgent {
    async fn begin_authentication(
        &self,
        action_id: &str,
        message: &str,
        _icon_name: &str,
        _details: HashMap<String, String>,
        cookie: &str,
        _identities: Vec<Subject>,
    ) -> Result<(), zbus::fdo::Error> {
        let exe =
            std::env::current_exe().map_err(|_| zbus::fdo::Error::Failed("Missing Exe".into()))?;

        println!(
            "\n[Daemon] Incoming D-Bus Request! Spawning UI prompt for: {}",
            action_id
        );

        let mut child = tokio::process::Command::new(exe)
            .arg("--ui")
            .arg(action_id)
            .arg(message)
            .stderr(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        if output.status.success() {
            let password = String::from_utf8_lossy(&output.stdout).trim().to_string();

            if !password.is_empty() {
                // --- FIX: DYNAMIC USER EXTRACTION ---
                let mut target_user = String::from("root"); // Default fallback

                if let Some(identity) = _identities.first() {
                    if identity.subject_kind == "unix-user" {
                        if let Some(uid_val) = identity.subject_details.get("uid") {
                            // Explicitly try to unwrap the zvariant value as a u32
                            if let Ok(uid) = u32::try_from(uid_val) {
                                let uid_str = uid.to_string();

                                if let Ok(output) = std::process::Command::new("id")
                                    .arg("-nu")
                                    .arg(&uid_str)
                                    .output()
                                {
                                    if output.status.success() {
                                        target_user = String::from_utf8_lossy(&output.stdout)
                                            .trim()
                                            .to_string();
                                    }
                                }
                            } else {
                                println!("[Daemon] Warning: UID variant was not a valid u32.");
                            }
                        }
                    }
                }

                println!(
                    "[Daemon] Target user determined as: '{}'. Forwarding to PAM...",
                    target_user
                );

                let mut helper =
                    tokio::process::Command::new("/usr/lib/polkit-1/polkit-agent-helper-1")
                        .arg(&target_user)
                        .env("POLKIT_AGENT_HELPER_1_COOKIE", cookie)
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::piped()) // Capture stdout to sequence the prompt
                        .stderr(std::process::Stdio::inherit())
                        .spawn()
                        .map_err(|e| {
                            zbus::fdo::Error::Failed(format!("Failed to spawn helper: {}", e))
                        })?;

                if let (Some(mut helper_out), Some(mut helper_in)) =
                    (helper.stdout.take(), helper.stdin.take())
                {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};

                    let mut buf = [0u8; 1024];

                    // 1. Wait for PAM to initialize and prompt for the password.
                    // This ensures the input buffer isn't flushed after we write to it.
                    if let Ok(bytes_read) = helper_out.read(&mut buf).await {
                        let output_str = String::from_utf8_lossy(&buf[..bytes_read]);

                        if output_str.contains("PAM_PROMPT_ECHO_OFF") {
                            // 2. Now that PAM is actively listening, send the password
                            let _ = helper_in
                                .write_all(format!("{}\n", password).as_bytes())
                                .await;
                            let _ = helper_in.flush().await;
                        }
                    }

                    // 3. Keep reading the remaining output so the helper doesn't stall,
                    // and keep stdin alive until PAM finishes processing.
                    let mut final_output = String::new();
                    let _ = helper_out.read_to_string(&mut final_output).await;
                }

                let helper_status = helper
                    .wait()
                    .await
                    .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

                if helper_status.success() {
                    println!("[Daemon] Authentication verified successfully.");
                } else {
                    println!("[Daemon] Authentication rejected by PAM.");
                }
            }
        } else {
            println!("[Daemon] UI prompt cancelled or failed.");
        }

        Ok(())
    }

    async fn cancel_authentication(&self, _cookie: &str) -> Result<(), zbus::fdo::Error> {
        println!("[Daemon] Request cancelled by authoritative daemon.");
        Ok(())
    }
}
fn get_true_session_id() -> String {
    // Attempt to extract the true logind session from the cgroup v2 tree
    if let Ok(cgroup) = std::fs::read_to_string("/proc/self/cgroup") {
        if let Some(scope) = cgroup
            .split('/')
            .find(|s| s.starts_with("session-") && s.ends_with(".scope"))
        {
            return scope
                .trim_start_matches("session-")
                .trim_end_matches(".scope")
                .to_string();
        }
    }

    // Fallback to the environment variable if cgroup parsing fails
    std::env::var("XDG_SESSION_ID").unwrap_or_else(|_| {
        eprintln!("WARNING: Could not determine true logind session.");
        String::new()
    })
}
async fn run_daemon() -> Result<(), Box<dyn Error>> {
    let connection = connection::Connection::system().await?;
    let agent_path = "/org/custom/PolkitAgent";

    connection
        .object_server()
        .at(agent_path, PolkitAgent)
        .await?;

    // Get the guaranteed session ID
    let session_id = get_true_session_id();
    println!("Registering agent under logind session: {}", session_id);

    let mut details = HashMap::new();
    details.insert(
        "session-id".to_string(),
        zbus::zvariant::Value::from(session_id.as_str())
            .try_to_owned()
            .unwrap(),
    );

    let subject = Subject {
        subject_kind: "unix-session".to_string(),
        subject_details: details,
    };

    let authority_proxy = AuthorityProxy::new(&connection).await?;

    // Register the agent for the entire graphical session
    authority_proxy
        .register_authentication_agent(&subject, "en_US.UTF-8", agent_path)
        .await?;

    println!("Session D-Bus Agent registered successfully. Holding event loop...");
    std::future::pending::<()>().await;

    Ok(())
}

// ---- Transient UI Application ----

struct AgentUI {
    action_id: String,
    message: String,
    password_input: String,
}

#[derive(Debug, Clone)]
enum Message {
    PasswordChanged(String),
    SubmitAuthentication,
    CancelAuthentication,
}

impl AgentUI {
    fn title(&self) -> String {
        String::from("System Authentication Required")
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PasswordChanged(val) => {
                self.password_input = val;
            }
            Message::SubmitAuthentication => {
                // Print the raw string into the pipe for the parent tokio process
                print!("{}", self.password_input);
                let _ = std::io::Write::flush(&mut std::io::stdout());
                std::process::exit(0);
            }
            Message::CancelAuthentication => {
                std::process::exit(1);
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        column![
            text("Authentication Required").size(22),
            text(format!("Action: {}", self.action_id)).size(13),
            text(&self.message).size(15),
            text_input("Enter Password", &self.password_input)
                .on_input(Message::PasswordChanged)
                .on_submit(Message::SubmitAuthentication)
                .secure(true),
            row![
                button("Authenticate").on_press(Message::SubmitAuthentication),
                button("Cancel").on_press(Message::CancelAuthentication),
            ]
            .spacing(12)
        ]
        .spacing(12)
        .padding(24)
        .align_x(Alignment::Center)
        .into()
    }
}

fn run_ui(action_id: String, message: String) -> iced::Result {
    // 1. The boot closure comes first. It takes 0 arguments and returns (State, Task).
    // The variables are cloned inside the closure to satisfy the `Fn` trait bound.
    iced::application(
        move || {
            (
                AgentUI {
                    action_id: action_id.clone(),
                    message: message.clone(),
                    password_input: String::new(),
                },
                Task::none(),
            )
        },
        AgentUI::update,
        AgentUI::view,
    )
    // 2. Chain the title handler
    .title(AgentUI::title)
    // 3. Trigger the loop natively
    .run()
}

// ---- Bootloader ----

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "--ui" {
        let action_id = args
            .get(2)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());
        let message = args
            .get(3)
            .cloned()
            .unwrap_or_else(|| "Authentication request".to_string());
        let _ = run_ui(action_id, message);
        return Ok(());
    }

    // Parent Process: Create a dedicated local tokio runtime so it never tangles with Iced.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        if let Err(e) = run_daemon().await {
            eprintln!("Daemon crashed: {}", e);
        }
    });

    Ok(())
}
