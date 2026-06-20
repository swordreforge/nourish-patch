//! Preset-independent config file text generators: xdg-portal preference, PAM
//! lock policy, and the polkit agent service.

/// xdg-desktop-portal preference (modeled on .reference/references/installer-artifact/xdg-portal).
pub fn portals_conf() -> String {
    "[preferred]\ndefault=gtk\n".to_string()
}

/// PAM service for the lock screen (from installation-y5-lock). Used only if the
/// staged template file is absent.
pub fn pam_y5_lock() -> String {
    "# /etc/pam.d/y5-lock\n\
     # PAM service for the y5 compositor's lock screen.\n\
     # RHEL / Fedora / Arch / openSUSE:\n\
     auth     include    system-auth\n\
     account  include    system-auth\n"
        .to_string()
}

/// systemd user service for the polkit authentication agent (new — none shipped).
pub fn polkit_service() -> String {
    "# ~/.config/systemd/user/y5-polkit-agent.service\n\
     [Unit]\n\
     Description=Y5 polkit authentication agent\n\
     After=graphical-session.target\n\
     PartOf=graphical-session.target\n\
     \n\
     [Service]\n\
     Type=simple\n\
     ExecStart=/usr/local/bin/y5-polkit-agent\n\
     Restart=on-failure\n\
     \n\
     [Install]\n\
     WantedBy=graphical-session.target\n"
        .to_string()
}
