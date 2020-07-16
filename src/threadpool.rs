use std::error::Error;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

#[allow(dead_code)]
type Job = Box<dyn FnOnce(usize) + Send + 'static>;

#[allow(dead_code)]
pub enum Message {
    NewJob(Job),
    Terminate,
}

#[allow(dead_code)]
struct Worker {
    wid: usize,
    thread: Option<thread::JoinHandle<()>>,
}

#[allow(dead_code)]
impl Worker {
    fn new(wid: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let msg = {
                let guard = receiver.lock().unwrap();
                match guard.recv() {
                    Ok(j) => j,
                    Err(_) => break,
                }
            };
            match msg {
                Message::NewJob(job) => job(wid),
                Message::Terminate => break,
            };
        });
        Worker {
            wid,
            thread: Some(thread),
        }
    }
    fn wait(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

#[allow(dead_code)]
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

#[allow(dead_code)]
impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let size = std::cmp::max(1, size);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let workers = (0..size)
            .map(|wid| Worker::new(wid, receiver.clone()))
            .collect();
        ThreadPool { workers, sender }
    }
    pub fn execute<F>(&self, f: F) -> Result<(), impl Error>
    where
        F: FnOnce(usize) + Send + 'static,
    {
        self.sender.send(Message::NewJob(Box::new(f)))
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }
        for worker in &mut self.workers {
            worker.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn threadpool_test() {
        use std::time::Duration;
        let workers = 4;
        let tasks = 6;
        let steps = 100;
        let tp = ThreadPool::new(workers);
        for i in 0..tasks {
            tp.execute(move |wid| {
                for j in 0..steps {
                    println!("Worker {} | Task {} | Step {}", wid, i, j);
                    thread::sleep(Duration::from_millis(10));
                }
            })
            .unwrap();
        }
        println!("Waiting for threadpool to terminate");
        drop(tp);
        println!("Threadpool terminated");
    }
}
