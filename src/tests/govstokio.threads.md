
Go's goroutines and Rust's Tokio tasks are both designed to handle concurrency efficiently, but they achieve this in different ways due to their underlying architectures and design philosophies. Here's how they differ in terms of blocking operations and I/O.
Rust tokio threads are used to execute none blocking asynchronous io tasks so instead of blocking 
them we should wait for the io to completes while other parts of the code are being executed concurrently  
however we can lock mutex or rwlock in a separate io thread to avoid blocking the entire program. 
leveraging the power of cpu threads is great for executing intensive cpu loads and tasks like 
cryptography operations which requires to block the whole thread for a while until the math is done. 
Go threads or goroutines are lightweight threads used to execute tasks in a blocking way that it 
uses netpoller to handle blocking io operations in a way that it doesn't block the entire program
the thread waits for the io to gets completed while the scheduler is scheudling other goroutines on 
a different OS thread to execute them concurrently without blocking program.
**Tokio**: none blokcing async io in a lightweigh thread of execution using an event loop.
**Golang**: blocking io in a lightweight thread without blocking the entire app using netpoller.
**blocking use case for both**: locking mutex in a thread to avoid data being mutated by other threads at the same time; if the lock is busy the caller thread gets blocked.
waiting means please block the thread so i can get the result but executing the task in the background light io thread means don't wait and just pause the execution in there and go execute other tasks, waiting in golang requires the thread to gets blocked but in tokio will be done a none blocking way:

### Goroutines in Go:
- **Multiplexing onto OS Threads:** Goroutines are lightweight and managed by the Go runtime. They are multiplexed onto a smaller number of OS threads. If a goroutine blocks, such as when waiting for I/O, the Go runtime can schedule other goroutines to run on different OS threads.
  
- **Blocking I/O:** Go uses blocking I/O operations in a way that doesn't block the entire program. When a goroutine performs a blocking I/O operation, such as reading from a file or network socket, it may cause the underlying OS thread to block. However, the Go runtime detects this and can schedule other goroutines on a different OS thread. This means that while one goroutine is waiting for I/O, others can continue executing without being blocked.

- **I/O Handling:** The Go runtime uses a mechanism called **netpoller** to efficiently manage I/O operations. When a goroutine performs a blocking I/O operation, the runtime uses the netpoller to put the goroutine to sleep, allowing other goroutines to run. When the I/O operation is ready, the goroutine is awakened and resumes execution. This allows Go to use blocking I/O in a way that doesn't block the entire application.

### Tokio Tasks in Rust:
- **Non-blocking I/O:** Tokio is an asynchronous runtime in Rust that is designed to use non-blocking I/O operations. When a Tokio task performs I/O, it uses the `async` and `await` keywords to pause execution until the I/O operation is ready. During this time, the task does not block the OS thread, allowing other tasks to run.

- **Event-driven Model:** Tokio uses an event-driven model, where tasks are executed on an event loop. When a task performs a non-blocking I/O operation, it yields control back to the event loop, which can then run other tasks. When the I/O operation is ready, the event loop wakes up the task, allowing it to continue executing.

- **No OS-level Blocking:** Since Tokio tasks are entirely non-blocking, there is no need for OS threads to block. The entire system is designed to maximize concurrency by avoiding blocking operations at the OS level.

### Key Differences:
- **Blocking vs Non-blocking I/O:** Go's goroutines can perform blocking I/O without blocking the entire program because the Go runtime handles the scheduling of goroutines across multiple OS threads. Tokio, on the other hand, relies on non-blocking I/O and an event loop to manage concurrency.

- **Runtime Overhead:** Go's approach with goroutines and blocking I/O might be easier to work with, as it abstracts away the complexities of non-blocking I/O. However, it can involve more runtime overhead due to the need to manage OS threads and context switching. Tokio, with its non-blocking I/O model, is more efficient in terms of resource usage but requires the developer to write more complex asynchronous code.

### Conclusion:
While Go's goroutines can handle blocking I/O without freezing the entire program, they do so by relying on the Go runtime's ability to manage and schedule OS threads. This is different from Tokio's approach, where tasks are non-blocking by design, and concurrency is achieved through an event-driven model. Both approaches are effective, but they cater to different programming paradigms and use cases.

### What Does "Pausing" Mean?

When you `await` a future in Rust, the following happens:

1. **Suspension of the Current Task**: The current asynchronous function (or "task") is suspended at the point where the `await` is called. This means that the function will not proceed to the next line of code until the future being awaited is ready to yield a result.

2. **Non-blocking Wait**: This suspension does **not** block the entire thread. Instead, it allows the runtime (e.g., the Tokio runtime) to schedule and run other asynchronous tasks or operations on the same thread. The runtime can continue executing other tasks in the same thread while the current task is waiting.

3. **Event Loop**: The Tokio runtime, or any async runtime, works on an event loop model. When you `await` a future, the event loop is notified that the current task is not ready to continue. The event loop then picks another task from the queue and runs it. When the awaited future is ready (e.g., when an I/O operation completes or a timer expires), the runtime resumes the suspended task exactly where it left off.

4. **Resumption**: Once the awaited future is ready, the task is resumed, and the code after the `await` is executed. This happens in the same thread where the task was originally running unless the runtime decides to move it to another thread (which usually doesn't happen unless you use specific APIs).

### Key Points About `await`:

- **Non-blocking**: The key aspect of `await` is that it doesn't block the thread. Instead, it allows other tasks to run in that thread. If you had multiple async tasks running, they could be interleaved by the runtime without any of them blocking the others.

- **Cooperative Multitasking**: The async model in Rust is based on cooperative multitasking, where tasks yield control at certain points (like when `await` is called) so that other tasks can be scheduled.

- **Single-Threaded Context**: If you're using a single-threaded async runtime, all tasks run on the same thread, but they don't block each other because of this cooperative nature. If you're using a multi-threaded runtime, tasks can be moved between threads, but the principle remains the same.

### Practical Example:

Here's an analogy:

- Imagine you have several workers (async tasks) who are all using the same desk (thread). When one worker needs to wait for a long operation (like fetching data), they get up from the desk (the task is suspended) and let another worker sit down and use the desk (another task runs).
- The first worker doesn't block the desk (the thread) while waiting—they're effectively pausing their work but allowing others to use the resources.

### Misunderstanding About Thread Blocking:

`await` **does not block** the thread like a traditional blocking operation (`std::thread::sleep` or I/O blocking). Instead, it allows the runtime to manage other tasks while waiting. The thread is free to do other work until the awaited future completes.

### Summary:

- **Pausing**: When we say `await` "pauses" the execution, it means the current async function is suspended until the awaited future is ready, but this suspension is non-blocking.
- **Runtime Flexibility**: The runtime can continue running other tasks on the same thread, making full use of the available resources without any blocking, hence the term "non-blocking wait."

This allows asynchronous programs to be efficient and responsive, as multiple tasks can progress concurrently without traditional thread blocking.