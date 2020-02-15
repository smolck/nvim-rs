use nvim_rs::rpc::handler::Dummy as DummyHandler;

#[cfg(feature = "use_tokio")]
use tokio::test as atest;
#[cfg(feature = "use_tokio")]
use nvim_rs::create::tokio as create;

#[cfg(feature = "use_async-std")]
use async_std::test as atest;
#[cfg(feature = "use_async-std")]
use nvim_rs::create::async_std as create;

use std::{
  process::Command,
  thread::sleep,
  time::{Duration, Instant},
};

#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use tempdir::TempDir;

const NVIMPATH: &str = "neovim/build/bin/nvim";
const HOST: &str = "127.0.0.1";
const PORT: u16 = 6666;

#[atest]
async fn can_connect_via_tcp() {
  let listen = HOST.to_string() + ":" + &PORT.to_string();

  let mut child = Command::new(NVIMPATH)
    .args(&["-u", "NONE", "--headless", "--listen", &listen])
    .spawn()
    .expect("Cannot start neovim");

  // wait at most 1 second for neovim to start and create the tcp socket
  let start = Instant::now();

  let (nvim, _io_handle, _method_handle) = loop {
    sleep(Duration::from_millis(100));

    let handler = DummyHandler::new();
    if let Ok(r) = create::new_tcp(&listen, handler).await {
      break r;
    } else {
      if Duration::from_secs(1) <= start.elapsed() {
        panic!("Unable to connect to neovim via tcp at {}", listen);
      }
    }
  };

  let servername = nvim
    .get_vvar("servername")
    .await
    .expect("Error retrieving servername from neovim");

  child.kill().expect("Could not kill neovim");

  assert_eq!(&listen, servername.as_str().unwrap());
}

#[cfg(unix)]
#[atest]
async fn can_connect_via_unix_socket() {
  let dir = TempDir::new("neovim-lib.test")
    .expect("Cannot create temporary directory for test.");

  let socket_path = dir.path().join("unix_socket");

  let mut child = Command::new(NVIMPATH)
    .args(&["-u", "NONE", "--headless"])
    .env("NVIM_LISTEN_ADDRESS", &socket_path)
    .spawn()
    .expect("Cannot start neovim");

  // wait at most 1 second for neovim to start and create the socket
  {
    let start = Instant::now();
    let one_second = Duration::from_secs(1);
    loop {
      sleep(Duration::from_millis(100));

      if let Ok(_) = std::fs::metadata(&socket_path) {
        break;
      }

      if one_second <= start.elapsed() {
        panic!(format!("neovim socket not found at '{:?}'", &socket_path));
      }
    }
  }

  let handler = DummyHandler::new();
  let (nvim, _io_handle, _method_handle) = create::new_unix_socket(&socket_path, handler)
    .await
    .expect(&format!(
      "Unable to connect to neovim's unix socket at {:?}",
      &socket_path
    ));

  let servername = nvim
    .get_vvar("servername")
    .await
    .expect("Error retrieving servername from neovim")
    .as_str()
    .unwrap()
    .to_string();

  child.kill().expect("Could not kill neovim");

  assert_eq!(socket_path, Path::new(&servername));
}
