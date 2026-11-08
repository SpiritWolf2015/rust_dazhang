//! 极简同步 Future executor（零依赖）。
//!
//! `wgpu` 的 `request_adapter` / `request_device` 返回 `impl Future`，而本机
//! 离线缓存没有 `pollster` / `futures` 可用；这里用 `std::task::Wake` 加线程
//! park/unpark 实现一个十几行的阻塞执行器，只用于初始化阶段，不参与帧循环。

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct ThreadWaker(std::thread::Thread);

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

/// 在当前线程阻塞运行 `future` 直到完成。
pub fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            // 允许虚假唤醒：醒来后重新 poll 即可。
            Poll::Pending => std::thread::park(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_ready_future() {
        let value = block_on(async { 7u32 });
        assert_eq!(value, 7);
    }

    #[test]
    fn resolves_pending_then_ready_future() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::task::Poll;

        let ready = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ready);
        let value = block_on(std::future::poll_fn(move |cx| {
            if flag.load(Ordering::SeqCst) {
                Poll::Ready(11u32)
            } else {
                flag.store(true, Ordering::SeqCst);
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }));
        assert_eq!(value, 11);
    }
}
