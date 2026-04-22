use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::gh;
use crate::model::{PrDetail, PrSummary};

pub enum Request {
    FetchList { limit: u32 },
    FetchDetail { number: u32 },
    Shutdown,
}

pub enum Response {
    List(Result<Vec<PrSummary>, String>),
    Detail { number: u32, result: Result<PrDetail, String> },
}

pub struct Worker {
    pub tx: Sender<Request>,
    pub rx: Receiver<Response>,
}

impl Worker {
    pub fn spawn() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<Request>();
        let (res_tx, res_rx) = mpsc::channel::<Response>();

        let viewer = gh::viewer_login().ok();

        thread::spawn(move || {
            while let Ok(req) = req_rx.recv() {
                match req {
                    Request::Shutdown => break,
                    Request::FetchList { limit } => {
                        let result =
                            gh::list_prs(limit, viewer.as_deref()).map_err(|e| format!("{e:#}"));
                        let _ = res_tx.send(Response::List(result));
                    }
                    Request::FetchDetail { number } => {
                        let result =
                            gh::view_pr(number, viewer.as_deref()).map_err(|e| format!("{e:#}"));
                        let _ = res_tx.send(Response::Detail { number, result });
                    }
                }
            }
        });

        Worker {
            tx: req_tx,
            rx: res_rx,
        }
    }

    pub fn request_list(&self, limit: u32) {
        let _ = self.tx.send(Request::FetchList { limit });
    }

    pub fn request_detail(&self, number: u32) {
        let _ = self.tx.send(Request::FetchDetail { number });
    }

    pub fn shutdown(&self) {
        let _ = self.tx.send(Request::Shutdown);
    }
}
