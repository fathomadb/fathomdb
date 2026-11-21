#define _POSIX_C_SOURCE 200809L
#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <stdio.h>
#include <unistd.h>
static void *remaining_thread(void *unused) {
    (void)unused;
    char target[4096];
    while (readlink("/proc/self/exe", target, sizeof target) >= 0) sched_yield();
    if (errno != ENOENT) _exit(2);
    if (write(STDOUT_FILENO, "EXE_GONE\n", 9) != 9) _exit(3);
    char signal;
    if (read(STDIN_FILENO, &signal, 1) != 1) _exit(4);
    return NULL;
}
int main(void) {
    pthread_t worker;
    if (pthread_create(&worker, NULL, remaining_thread, NULL)) return 5;
    if (write(STDOUT_FILENO, "READY\n", 6) != 6) return 6;
    char signal;
    if (read(STDIN_FILENO, &signal, 1) != 1) return 7;
    pthread_exit(NULL);
}
