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

        support_priority: { any(elf,coff) },

        constructor_destructor: { any(elf,coff,mach_o) },

    }
}
