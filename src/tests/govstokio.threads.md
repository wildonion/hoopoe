
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
**blocking use case for both**: locking mutex in a thread to avoid data being mutated by other threads at the same time.
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