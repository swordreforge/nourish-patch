use zbus::blocking::{Connection, Proxy};

pub struct VtSwitcher {
    conn: Connection,
}

impl VtSwitcher {
    pub fn new() -> zbus::Result<Self> {
        Ok(Self {
            conn: Connection::system()?,
        })
    }

    // This is not the native way to suspend per smithay api and other os stuff.
    // pub fn switch_to(&self, vt: u32) -> zbus::Result<()> {
    //     if !(1..=63).contains(&vt) {
    //         return Err(zbus::Error::InvalidField);
    //     }
    //     let proxy = Proxy::new(
    //         &self.conn,
    //         "org.freedesktop.login1",
    //         "/org/freedesktop/login1/seat/seat0",
    //         "org.freedesktop.login1.Seat",
    //     )?;
    //     proxy.call_method("SwitchTo", &(vt,))?;
    //     Ok(())
    // }

    pub fn suspend(&self) -> zbus::Result<()> {
        let proxy = Proxy::new(
            &self.conn,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        )?;
        proxy.call_method("Suspend", &(true))?;
        Ok(())
    }
}
