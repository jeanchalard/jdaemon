use dbus::blocking::{Connection, BlockingSender};
use dbus::message::Message;
use dbus_crossroads::{Crossroads, Context, MethodErr};
use std::error::Error;
use std::time::Duration;

const DBUS_SERVICE_NAME : &str = "org.j.Daemon";
const DBUS_OBJECT_PATH : &str = "/org/j/Daemon/ScreenSaver";

const SCREENSAVER_SERVICE : &str = "org.freedesktop.ScreenSaver";
const SCREENSAVER_PATH : &str = "/org/freedesktop/ScreenSaver";

enum Status { Active, Inhibited(u32) }

struct Daemon {
  connection : Connection,
  status : Status,
}

impl Daemon {
  #[allow(non_snake_case)]
  fn methodInhibit(ctx : &mut Context, daemon : &mut Daemon, _args : ()) -> Result<(&'static str,), MethodErr> {
    match daemon.status {
      Status::Active => {
        let cookieMsg = daemon.connection.send_with_reply_and_block(Message::call_with_args(
            SCREENSAVER_SERVICE,
            SCREENSAVER_PATH,
            SCREENSAVER_SERVICE,
            "Inhibit", ("JDaemon" /* application */, "Manual inhibit" /* reason */,)),
          Duration::from_millis(5_000))?;
        match cookieMsg.get1::<u32>() {
          Some(cookie) => { daemon.status = Status::Inhibited(cookie); println!("Screen blank inhibited, cookie {}", cookie) },
          None => println!("Sent inhibit, error"),
        }
      },
      Status::Inhibited(cookie) => println!("Screen blank was already inhibited with cookie {}", cookie),
    }
    return Daemon::methodGetStatus(ctx, daemon, ())
  }

  #[allow(non_snake_case)]
  fn methodUninhibit(ctx : &mut Context, daemon : &mut Daemon, _args : ()) -> Result<(&'static str,), MethodErr> {
    match daemon.status {
      Status::Inhibited(cookie) => {
        daemon.connection.send_with_reply_and_block(Message::call_with_args(
            SCREENSAVER_SERVICE,
            SCREENSAVER_PATH,
            SCREENSAVER_SERVICE,
            "UnInhibit", (cookie,)),
          Duration::from_millis(5_000))?;
        daemon.status = Status::Active;
        println!("Screen blank uninhibited");
      },
      Status::Active => println!("Screen blank was already active"),
    }
    return Daemon::methodGetStatus(ctx, daemon, ())
  }

  #[allow(non_snake_case)]
  fn methodToggleInhibit(ctx : &mut Context, daemon : &mut Daemon, _args : ()) -> Result<(&'static str,), MethodErr> {
    return match daemon.status {
      Status::Active => Daemon::methodInhibit(ctx, daemon, ()),
      Status::Inhibited(_) => Daemon::methodUninhibit(ctx, daemon, ()),
    }
  }

  #[allow(non_snake_case)]
  fn methodGetStatus(_ctx : &mut Context, daemon : &mut Daemon, _args : ()) -> Result<(&'static str,), MethodErr> {
    match daemon.status {
      Status::Active => Ok(("Active",)),
      Status::Inhibited(_) => Ok(("Inhibited",)),
    }
  }

  fn new() -> Daemon {
    let conn = Connection::new_session();
    match conn {
      Ok(val) => Daemon { connection : val, status : Status::Active },
      Err(_) => std::process::exit(1),
    }
  }
}

fn main() -> Result<(), Box<dyn Error>> {
  // Connect to the D-Bus session bus
  let conn = Connection::new_session()?;
  conn.request_name(DBUS_SERVICE_NAME, true /* allow_replacement */, true /* replace_existing */, false /* do_not_queue */)?;

  let mut cr = Crossroads::new();
  let obj = cr.register("org.j.Daemon.ScreenSaver", |b| {
    b.method("Inhibit", (), ("status",), Daemon::methodInhibit);
    b.method("UnInhibit", (), ("status",), Daemon::methodUninhibit);
    b.method("ToggleInhibit", (), ("status",), Daemon::methodToggleInhibit);
    b.method("getStatus", (), ("status",), Daemon::methodGetStatus);
  });

  cr.insert(DBUS_OBJECT_PATH, &[obj], Daemon::new());
  cr.serve(&conn)?;
  unreachable!();
}
