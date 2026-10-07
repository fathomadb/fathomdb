[Thread debugging using libthread_db enabled]
Using host libthread_db library "/lib/aarch64-linux-gnu/libthread_db.so.1".

Thread 1 "python" received signal SIGSEGV, Segmentation fault.
0x0000fffff33a0314 in ?? () from /lib/aarch64-linux-gnu/libcuda.so
	Using the running image of child Thread 0xfffff7ff3020 (LWP 3050998).
Program stopped at 0xfffff33a0314.
It stopped with signal SIGSEGV, Segmentation fault.
#0  0x0000fffff33a0314 in ?? () from /lib/aarch64-linux-gnu/libcuda.so
#1  0x0000fffff3212c2c in ?? () from /lib/aarch64-linux-gnu/libcuda.so
#2  0x0000fffff62ccf78 in cudarc::driver::safe::core::CudaStream::wait () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#3  0x0000fffff612166c in <cudarc::driver::safe::core::CudaSlice<T> as core::ops::drop::Drop>::drop () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#4  0x0000fffff612a4b4 in core::ptr::drop_in_place<cudarc::driver::safe::core::CudaSlice<f32>> () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#5  0x0000fffff612b078 in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#6  0x0000fffff612acbc in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#7  0x0000fffff5b8cedc in core::ptr::drop_in_place<candle_transformers::models::bert::BertModel> () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#8  0x0000fffff5b8d710 in core::ptr::drop_in_place<fathomdb_embedder::candle_bge::CandleBgeEmbedder> () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#9  0x0000fffff5dbf5a8 in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#10 0x0000fffff5dbf0f0 in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#11 0x0000fffff5dbf33c in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#12 0x0000fffff5dbdb70 in core::ptr::drop_in_place<fathomdb_engine::projection_runtime::ProjectionRuntimeShared> () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#13 0x0000fffff5dbf4a0 in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#14 0x0000fffff5bc7ab4 in core::ptr::drop_in_place<fathomdb_engine::projection_runtime::ProjectionRuntime> () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#15 0x0000fffff5bc84b0 in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#16 0x0000fffff5bcafec in _ZN4core3ptr50drop_in_place$LT$fathomdb_py..engine..PyEngine$GT$17h7f0a45857f485777E.llvm.7707358796876329471 () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#17 0x0000fffff5bc9d0c in <pyo3::pycell::impl_::PyStaticClassObject<T> as pyo3::pycell::impl_::PyClassObjectBaseLayout<T>>::tp_dealloc () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#18 0x0000fffff5b7575c in pyo3::impl_::trampoline::trampoline_unraisable () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#19 0x0000fffff5b75c34 in pyo3::impl_::pyclass::tp_dealloc () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#20 0x0000aaaaaae14f78 in subtype_dealloc ()
#21 0x0000aaaaaadc59dc in cell_dealloc ()
#22 0x0000aaaaaae7f524 in _PyFrame_ClearExceptCode ()
#23 0x0000aaaaaae463e8 in _PyEval_EvalFrameDefault ()
#24 0x0000aaaaaaf84a30 in PyEval_EvalCode ()
#25 0x0000aaaaaafbb064 in run_mod.llvm ()
#26 0x0000aaaaaad03ff0 in pyrun_file ()
#27 0x0000aaaaaad03390 in _PyRun_SimpleFileObject ()
#28 0x0000aaaaaad02f74 in _PyRun_AnyFileObject ()
#29 0x0000aaaaaad10130 in pymain_run_file_obj ()
#30 0x0000aaaaaad0fe68 in pymain_run_file ()
#31 0x0000aaaaaafcf0dc in Py_RunMain ()
#32 0x0000aaaaaafcf604 in pymain_main.llvm ()
#33 0x0000aaaaaaeb83b0 in main ()
  Id   Target Id                                            Frame 
* 1    Thread 0xfffff7ff3020 (LWP 3050998) "python"         0x0000fffff33a0314 in ?? () from /lib/aarch64-linux-gnu/libcuda.so
  22   Thread 0xffffecade100 (LWP 3051021) "cuda-EvtHandlr" 0x0000fffff7dbfce0 in __GI___poll (fds=0xffffe8000c20, nfds=10, timeout=<optimized out>) at ../sysdeps/unix/sysv/linux/poll.c:41
  23   Thread 0xffffe7fff100 (LWP 3051022) "python"         __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0xffffe7ffe7d0, op=393, expected=0, futex_word=0xaaaab086fd80) at ./nptl/futex-internal.c:57

Thread 23 (Thread 0xffffe7fff100 (LWP 3051022) "python"):
#0  __futex_abstimed_wait_common64 (private=0, cancel=true, abstime=0xffffe7ffe7d0, op=393, expected=0, futex_word=0xaaaab086fd80) at ./nptl/futex-internal.c:57
#1  __futex_abstimed_wait_common (cancel=true, private=0, abstime=0xffffe7ffe7d0, clockid=-1, expected=0, futex_word=0xaaaab086fd80) at ./nptl/futex-internal.c:87
#2  __GI___futex_abstimed_wait_cancelable64 (futex_word=futex_word@entry=0xaaaab086fd80, expected=expected@entry=0, clockid=clockid@entry=0, abstime=abstime@entry=0xffffe7ffe7d0, private=private@entry=0) at ./nptl/futex-internal.c:139
#3  0x0000fffff7d5f9e4 in __pthread_cond_wait_common (abstime=0xffffe7ffe7d0, clockid=0, mutex=0xaaaab086a660, cond=0xaaaab086fd58) at ./nptl/pthread_cond_wait.c:503
#4  ___pthread_cond_timedwait64 (cond=0xaaaab086fd58, mutex=0xaaaab086a660, abstime=0xffffe7ffe7d0) at ./nptl/pthread_cond_wait.c:652
#5  0x0000fffff312180c in ?? () from /lib/aarch64-linux-gnu/libcuda.so
#6  0x0000fffff31ac0c4 in ?? () from /lib/aarch64-linux-gnu/libcuda.so
#7  0x0000fffff7d603c8 in start_thread (arg=0x80e940) at ./nptl/pthread_create.c:442
#8  0x0000fffff7dc9fdc in thread_start () at ../sysdeps/unix/sysv/linux/aarch64/clone.S:79

Thread 22 (Thread 0xffffecade100 (LWP 3051021) "cuda-EvtHandlr"):
#0  0x0000fffff7dbfce0 in __GI___poll (fds=0xffffe8000c20, nfds=10, timeout=<optimized out>) at ../sysdeps/unix/sysv/linux/poll.c:41
#1  0x0000fffff31af290 in ?? () from /lib/aarch64-linux-gnu/libcuda.so
#2  0x0000fffff324c620 in ?? () from /lib/aarch64-linux-gnu/libcuda.so
#3  0x0000fffff31ac0c4 in ?? () from /lib/aarch64-linux-gnu/libcuda.so
#4  0x0000fffff7d603c8 in start_thread (arg=0x80e940) at ./nptl/pthread_create.c:442
#5  0x0000fffff7dc9fdc in thread_start () at ../sysdeps/unix/sysv/linux/aarch64/clone.S:79

Thread 1 (Thread 0xfffff7ff3020 (LWP 3050998) "python"):
#0  0x0000fffff33a0314 in ?? () from /lib/aarch64-linux-gnu/libcuda.so
#1  0x0000fffff3212c2c in ?? () from /lib/aarch64-linux-gnu/libcuda.so
#2  0x0000fffff62ccf78 in cudarc::driver::safe::core::CudaStream::wait () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#3  0x0000fffff612166c in <cudarc::driver::safe::core::CudaSlice<T> as core::ops::drop::Drop>::drop () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#4  0x0000fffff612a4b4 in core::ptr::drop_in_place<cudarc::driver::safe::core::CudaSlice<f32>> () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#5  0x0000fffff612b078 in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#6  0x0000fffff612acbc in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#7  0x0000fffff5b8cedc in core::ptr::drop_in_place<candle_transformers::models::bert::BertModel> () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#8  0x0000fffff5b8d710 in core::ptr::drop_in_place<fathomdb_embedder::candle_bge::CandleBgeEmbedder> () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#9  0x0000fffff5dbf5a8 in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#10 0x0000fffff5dbf0f0 in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
#11 0x0000fffff5dbf33c in alloc::sync::Arc<T,A>::drop_slow () from $PS/venv-p/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
