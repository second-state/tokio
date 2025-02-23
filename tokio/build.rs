use autocfg::AutoCfg;

const CONST_THREAD_LOCAL_PROBE: &str = r#"
{
    thread_local! {
        static MY_PROBE: usize = const { 10 };
    }

    MY_PROBE.with(|val| *val)
}
"#;

const CONST_MUTEX_NEW_PROBE: &str = r#"
{
    static MY_MUTEX: ::std::sync::Mutex<i32> = ::std::sync::Mutex::new(1);
    *MY_MUTEX.lock().unwrap()
}
"#;

const AS_FD_PROBE: &str = r#"
{
    #[allow(unused_imports)]
    #[cfg(unix)]
    use std::os::unix::prelude::AsFd as _;
    #[allow(unused_imports)]
    #[cfg(windows)]
    use std::os::windows::prelude::AsSocket as _;
    #[allow(unused_imports)]
    #[cfg(target_os = "wasi")]
    use std::os::wasi::prelude::AsFd as _;
}
"#;

const TARGET_HAS_ATOMIC_PROBE: &str = r#"
{
    #[cfg(target_has_atomic = "ptr")]
    let _ = ();
}
"#;

const TARGET_ATOMIC_U64_PROBE: &str = r#"
{
    #[allow(unused_imports)]
    use std::sync::atomic::AtomicU64 as _;
}
"#;

fn main() {
    let mut enable_const_thread_local = false;
    let mut enable_target_has_atomic = false;
    let mut enable_const_mutex_new = false;
    let mut enable_as_fd = false;
    let mut target_needs_atomic_u64_fallback = false;

    match AutoCfg::new() {
        Ok(ac) => {
            // These checks prefer to call only `probe_rustc_version` if that is
            // enough to determine whether the feature is supported. This is
            // because the `probe_expression` call involves a call to rustc,
            // which the `probe_rustc_version` call avoids.

            // Const-initialized thread locals were stabilized in 1.59.
            if ac.probe_rustc_version(1, 60) {
                enable_const_thread_local = true;
            } else if ac.probe_rustc_version(1, 59) {
                // This compiler claims to be 1.59, but there are some nightly
                // compilers that claim to be 1.59 without supporting the
                // feature. Explicitly probe to check if code using them
                // compiles.
                //
                // The oldest nightly that supports the feature is 2021-12-06.
                if ac.probe_expression(CONST_THREAD_LOCAL_PROBE) {
                    enable_const_thread_local = true;
                }
            }

            // The `target_has_atomic` cfg was stabilized in 1.60.
            if ac.probe_rustc_version(1, 61) {
                enable_target_has_atomic = true;
            } else if ac.probe_rustc_version(1, 60) {
                // This compiler claims to be 1.60, but there are some nightly
                // compilers that claim to be 1.60 without supporting the
                // feature. Explicitly probe to check if code using them
                // compiles.
                //
                // The oldest nightly that supports the feature is 2022-02-11.
                if ac.probe_expression(TARGET_HAS_ATOMIC_PROBE) {
                    enable_target_has_atomic = true;
                }
            }

            // If we can't tell using `target_has_atomic`, tell if the target
            // has `AtomicU64` by trying to use it.
            if !enable_target_has_atomic && !ac.probe_expression(TARGET_ATOMIC_U64_PROBE) {
                target_needs_atomic_u64_fallback = true;
            }

            // The `Mutex::new` method was made const in 1.63.
            if ac.probe_rustc_version(1, 64) {
                enable_const_mutex_new = true;
            } else if ac.probe_rustc_version(1, 63) {
                // This compiler claims to be 1.63, but there are some nightly
                // compilers that claim to be 1.63 without supporting the
                // feature. Explicitly probe to check if code using them
                // compiles.
                //
                // The oldest nightly that supports the feature is 2022-06-20.
                if ac.probe_expression(CONST_MUTEX_NEW_PROBE) {
                    enable_const_mutex_new = true;
                }
            }

            // The `AsFd` family of traits were made stable in 1.63.
            if ac.probe_rustc_version(1, 64) {
                enable_as_fd = true;
            } else if ac.probe_rustc_version(1, 63) {
                // This compiler claims to be 1.63, but there are some nightly
                // compilers that claim to be 1.63 without supporting the
                // feature. Explicitly probe to check if code using them
                // compiles.
                //
                // The oldest nightly that supports the feature is 2022-06-16.
                if ac.probe_expression(AS_FD_PROBE) {
                    enable_as_fd = true;
                }
            }
        }

        Err(e) => {
            // If we couldn't detect the compiler version and features, just
            // print a warning. This isn't a fatal error: we can still build
            // Tokio, we just can't enable cfgs automatically.
            println!(
                "cargo:warning=tokio: failed to detect compiler features: {}",
                e
            );
        }
    }

    if !enable_const_thread_local {
        // To disable this feature on compilers that support it, you can
        // explicitly pass this flag with the following environment variable:
        //
        // RUSTFLAGS="--cfg tokio_no_const_thread_local"
        autocfg::emit("tokio_no_const_thread_local")
    }

    if !enable_target_has_atomic {
        // To disable this feature on compilers that support it, you can
        // explicitly pass this flag with the following environment variable:
        //
        // RUSTFLAGS="--cfg tokio_no_target_has_atomic"
        autocfg::emit("tokio_no_target_has_atomic")
    }

    if !enable_const_mutex_new {
        // To disable this feature on compilers that support it, you can
        // explicitly pass this flag with the following environment variable:
        //
        // RUSTFLAGS="--cfg tokio_no_const_mutex_new"
        autocfg::emit("tokio_no_const_mutex_new")
    }

    if !enable_as_fd {
        // To disable this feature on compilers that support it, you can
        // explicitly pass this flag with the following environment variable:
        //
        // RUSTFLAGS="--cfg tokio_no_as_fd"
        autocfg::emit("tokio_no_as_fd");
    }

    if target_needs_atomic_u64_fallback {
        // To disable this feature on compilers that support it, you can
        // explicitly pass this flag with the following environment variable:
        //
        // RUSTFLAGS="--cfg tokio_no_atomic_u64"
        autocfg::emit("tokio_no_atomic_u64")
    }

    let target = ::std::env::var("TARGET").unwrap_or_default();

    // We emit cfgs instead of using `target_family = "wasm"` that requires Rust 1.54.
    // Note that these cfgs are unavailable in `Cargo.toml`.
    if target.starts_with("wasm") {
        autocfg::emit("tokio_wasm");
        if target.contains("wasi") {
            autocfg::emit("tokio_wasi_wasmedge");
        } else {
            autocfg::emit("tokio_wasm_not_wasi");
        }
    }
}
