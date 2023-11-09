use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {

        elf: { any(
            target_os = "linux",
            target_os = "android",
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "netbsd",
            target_os = "openbsd",
            target_os = "solaris",
            target_os = "illumos",
            target_os = "emscripten",
            target_os = "haiku",
            target_os = "l4re",
            target_os = "fuchsia",
            target_os = "redox",
            target_os = "vxworks"
            )},

        coff: { target_os = "windows" },

        mach_o: { any(target_os = "macos", target_os = "ios") },

        debug_mode: { any(feature = "debug_order", debug_assertions) },

        //TODO: use crate linkme to implement priorities for mach_o
        support_priority: { any(elf,coff) },

        cxa_thread_at_exit: { any(
            target_os = "linux",
            target_os = "fushia",
            target_os = "redox",
            target_os = "emscripten" ,
            target_env = "gnu")},

        pthread_thread_at_exit: { all(any(unix,target_env="gnu"),not(cxa_thread_at_exit)) },

        coff_thread_at_exit: {all(coff,not(pthread_thread_at_exit))},

    }
}
