use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::fd::OwnedFd;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Serve real Git protocol requests: libgit2's file transport does not negotiate depth.
pub struct GitServer {
    address: SocketAddr,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl GitServer {
    pub fn new(repository: &Path) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = stopped.clone();
        let repository = repository.to_owned();
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                Command::new("git")
                    .args(["-c", "uploadpack.allowReachableSHA1InWant=true"])
                    .args(["daemon", "--inetd", "--export-all", "--timeout=10"])
                    .arg("--log-destination=stderr")
                    .arg("--init-timeout=10")
                    .arg(format!("--base-path={}", repository.display()))
                    .env("GIT_CONFIG_NOSYSTEM", "1")
                    .env("GIT_CONFIG_GLOBAL", "/dev/null")
                    .stdin(Stdio::from(OwnedFd::from(stream.try_clone().unwrap())))
                    .stdout(Stdio::from(OwnedFd::from(stream)))
                    .stderr(Stdio::null())
                    .status()
                    .expect("git daemon must be installed");
            }
        });
        Self {
            address,
            stopped,
            thread: Some(thread),
        }
    }

    pub fn url(&self) -> String {
        format!("git://{}/", self.address)
    }
}

impl Drop for GitServer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        self.thread.take().unwrap().join().unwrap();
    }
}
