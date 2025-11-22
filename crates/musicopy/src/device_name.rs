use std::sync::LazyLock;

#[cfg(target_os = "android")]
fn device_name_impl() -> anyhow::Result<String> {
    use jni::{JavaVM, objects::JString};

    let vm = unsafe { JavaVM::from_raw(ndk_context::android_context().vm().cast()) }
        .expect("ndk_context should be initialized");
    let mut env = vm.attach_current_thread()?;

    let build_class = env.find_class("android/os/Build")?;

    // https://developer.android.com/reference/android/os/Build.html#MANUFACTURER
    let manufacturer: JString<'_> = env
        .get_static_field(&build_class, "MANUFACTURER", "Ljava/lang/String;")?
        .l()?
        .into();
    let manufacturer_string = env.get_string(&manufacturer)?;

    // https://developer.android.com/reference/android/os/Build.html#PRODUCT
    let product: JString<'_> = env
        .get_static_field(&build_class, "PRODUCT", "Ljava/lang/String;")?
        .l()?
        .into();
    let product_string = env.get_string(&product)?;

    Ok(format!(
        "{} {}",
        manufacturer_string.to_string_lossy(),
        product_string.to_string_lossy()
    ))
}

#[cfg(target_os = "ios")]
fn device_name_impl() -> anyhow::Result<String> {
    // TODO
    Ok("iOS Device".into())
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn device_name_impl() -> anyhow::Result<String> {
    let device = whoami::devicename()
        .or_else(|_| whoami::hostname())
        .or_else(|_| whoami::distro())
        .unwrap_or_else(|_| whoami::platform().to_string());

    Ok(device)
}

static DEVICE_NAME: LazyLock<String> =
    LazyLock::new(|| device_name_impl().unwrap_or("Unknown".into()));

pub fn device_name() -> &'static str {
    &DEVICE_NAME
}

#[uniffi::export(name = "get_device_name")]
pub fn device_name_owned() -> String {
    DEVICE_NAME.clone()
}
