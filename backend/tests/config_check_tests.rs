use std::{
    fs,
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

const JWT_SECRET: &str = "config-check-test-key-at-least-32-bytes";
const REDIS_SECRET: &str = "config-check-redis-sensitive";
const ADMIN_SECRET: &str = "config-check-admin-sensitive";
const TWO_FACTOR_KEY: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";
const PRIVATE_VALUE: &str = "private-config-marker";

struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("guardian-config-{}", uuid::Uuid::now_v7()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_guardian-backend"));
        command
            .current_dir(&self.0)
            .env_clear()
            .env("JWT_SECRET", JWT_SECRET)
            .env("DATABASE_URL", "sqlite://missing-db/guardian.db?mode=rwc")
            .env("REDIS_URL", "redis://127.0.0.1:1")
            .env("REDIS_PASSWORD", REDIS_SECRET)
            .env("SEED_ADMIN_PASSWORD", ADMIN_SECRET)
            .env("LOG_OUT_DIR", "missing-logs")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // 保留覆盖率输出位置，避免插桩运行时在被测目录生成 profraw。
        if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
            command.env("LLVM_PROFILE_FILE", profile);
        }
        command
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn run(command: &mut Command) -> Output {
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!("configuration command did not exit: {output:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
    child.wait_with_output().unwrap()
}

fn assert_safe_output(output: &Output) {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for secret in [
        JWT_SECRET,
        REDIS_SECRET,
        ADMIN_SECRET,
        TWO_FACTOR_KEY,
        PRIVATE_VALUE,
        "startup configuration",
    ] {
        assert!(!text.contains(secret), "unexpected output: {output:?}");
    }
}

fn assert_success(output: &Output) {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("configuration check passed"), "{stdout}");
    assert!(
        stdout.contains("configured local xdb files only"),
        "{stdout}"
    );
    assert!(stdout.contains("no connectivity"), "{stdout}");
    assert_safe_output(output);
}

fn assert_failure(output: &Output, key: &str) {
    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(key),
        "{output:?}"
    );
    assert_safe_output(output);
}

fn write_xdb(path: &Path, version: u16) {
    // 空库仍有完整头部和向量索引；零指针表示没有地址记录。
    let mut bytes = vec![0; 256 + 256 * 256 * 8];
    bytes[16..18].copy_from_slice(&version.to_le_bytes());
    fs::write(path, bytes).unwrap();
}

#[test]
fn check_config_exits_without_connecting_writing_or_binding() {
    let sandbox = Sandbox::new();
    let redis = TcpListener::bind("127.0.0.1:0").unwrap();
    redis.set_nonblocking(true).unwrap();
    let http = TcpListener::bind("127.0.0.1:0").unwrap();
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env(
            "REDIS_URL",
            format!("redis://{}", redis.local_addr().unwrap()),
        )
        .env("SERVER_PORT", http.local_addr().unwrap().port().to_string()));
    assert_success(&output);
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 0);
    assert_eq!(
        redis.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn unreachable_redis_and_optional_unconfigured_files_are_accepted() {
    let sandbox = Sandbox::new();
    let redis = TcpListener::bind("127.0.0.1:0").unwrap();
    let redis_url = format!("redis://{}", redis.local_addr().unwrap());
    drop(redis);
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env("REDIS_URL", redis_url)
        .env("IP_LOCATION_DB_PATH", "   ")
        .env("IP_LOCATION_V6_DB_PATH", "")
        .env("TWO_FACTOR_KEY", ""));
    assert_success(&output);
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 0);
}

#[tokio::test]
async fn check_config_preserves_existing_database_logs_and_xdb_files() {
    use sqlx::Connection;

    let sandbox = Sandbox::new();
    let database_url = format!(
        "sqlite://{}?mode=rwc",
        sandbox.0.join("existing.db").display()
    );
    let mut connection = sqlx::SqliteConnection::connect(&database_url)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE untouched (value TEXT)")
        .execute(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();
    fs::create_dir(sandbox.0.join("logs")).unwrap();
    fs::write(sandbox.0.join("logs/app.log"), "existing log\n").unwrap();
    write_xdb(&sandbox.0.join("v4.xdb"), 4);
    write_xdb(&sandbox.0.join("v6.xdb"), 6);
    let paths = [
        "",
        "logs",
        "existing.db",
        "logs/app.log",
        "v4.xdb",
        "v6.xdb",
    ];
    let snapshot = || {
        paths
            .iter()
            .map(|path| {
                let path = sandbox.0.join(path);
                let metadata = fs::metadata(&path).unwrap();
                (
                    metadata.modified().unwrap(),
                    metadata.is_file().then(|| fs::read(&path).unwrap()),
                )
            })
            .collect::<Vec<_>>()
    };
    let before = snapshot();
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env("DATABASE_URL", database_url)
        .env("LOG_OUT_DIR", "logs")
        .env("IP_LOCATION_DB_PATH", "v4.xdb")
        .env("IP_LOCATION_V6_DB_PATH", "v6.xdb")
        .env("TWO_FACTOR_KEY", TWO_FACTOR_KEY));
    assert_success(&output);
    assert!(snapshot() == before, "database, logs or xdb files changed");
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 4);
    assert_eq!(fs::read_dir(sandbox.0.join("logs")).unwrap().count(), 1);
}

#[test]
fn invalid_configuration_exits_with_a_safe_key_and_reason() {
    let sandbox = Sandbox::new();
    for (key, value) in [
        ("JWT_SECRET", PRIVATE_VALUE),
        ("JWT_ACCESS_TOKEN_TTL_MINUTES", "0"),
        ("SERVER_HOST", PRIVATE_VALUE),
        ("SERVER_PORT", PRIVATE_VALUE),
        ("SERVER_PORT", "65536"),
        ("MIDDLEWARE_TRUSTED_PROXIES", PRIVATE_VALUE),
        (
            "MIDDLEWARE_CORS_ALLOWED_ORIGINS",
            "private-config-marker\nbad",
        ),
        ("MIDDLEWARE_HSTS_ENABLED", PRIVATE_VALUE),
        ("MIDDLEWARE_MAX_CONCURRENCY", "0"),
        ("TWO_FACTOR_KEY", PRIVATE_VALUE),
        ("LOG_FILTER", "info,target=private-config-marker"),
        ("LOG_UTC_OFFSET_HOUR", "127"),
        ("LOG_UTC_OFFSET_MINUTE", "60"),
        ("LOG_UTC_OFFSET_SECOND", "60"),
        ("LOG_ROTATION", PRIVATE_VALUE),
        ("LOG_MAX_LOG_FILES", "0"),
        ("LOG_BATCH_MAX_EVENTS", "0"),
        ("LOG_BATCH_FLUSH_INTERVAL_SECS", "0"),
        (
            "DATABASE_URL",
            "private-config-marker://user:private-config-marker@host/db",
        ),
        (
            "DATABASE_URL",
            "sqlite://private-config-marker.db?mode=private-config-marker",
        ),
        (
            "DATABASE_URL",
            "sqlite://existing.db?private-config-marker=secret",
        ),
        ("DATABASE_MIN_CONNECTIONS", "0"),
        ("DATABASE_SQLX_LOGGING", PRIVATE_VALUE),
        (
            "REDIS_URL",
            "redis://user:private-config-marker@localhost:invalid/0",
        ),
        ("REDIS_URL", "redis://localhost/private-config-marker"),
        (
            "REDIS_URL",
            "private-config-marker://user:private-config-marker@host",
        ),
        ("SEED_USERS", PRIVATE_VALUE),
        ("SEED_USERS", "user:name:email:private-config-marker"),
        ("CACHE_MAX_LOADS", "65"),
    ] {
        let output = run(sandbox.command().arg("--check-config").env(key, value));
        assert_failure(&output, key);
    }
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env_remove("JWT_SECRET"));
    assert_failure(&output, "JWT_SECRET");
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn non_unicode_required_value_does_not_leak_through_the_error_chain() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let sandbox = Sandbox::new();
    let mut value = PRIVATE_VALUE.as_bytes().to_vec();
    value.push(0xff);
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env("JWT_SECRET", OsString::from_vec(value)));
    assert_failure(&output, "JWT_SECRET");
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 0);
}

#[test]
fn valid_driver_options_and_credentials_are_parsed_without_resolution() {
    let sandbox = Sandbox::new();
    let redis_url = format!("redis://test-user:{REDIS_SECRET}@unreachable.invalid:6379/3");
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env("REDIS_URL", &redis_url)
        .env_remove("REDIS_PASSWORD")
        .env("DATABASE_URL", "sqlite://missing.db?mode=ro&cache=private"));
    assert_success(&output);
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env("REDIS_PASSWORD", format!("{REDIS_SECRET}#/ +:@")));
    assert_success(&output);
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 0);
}

#[test]
fn production_configuration_obeys_the_existing_runtime_restrictions() {
    let sandbox = Sandbox::new();
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env("APP_ENV", "production"));
    if cfg!(debug_assertions) {
        assert_failure(&output, "APP_ENV");
    } else {
        assert_success(&output);
    }
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env("APP_ENV", "production")
        .env("MIDDLEWARE_TRUSTED_PROXIES", "*"));
    assert_failure(&output, "MIDDLEWARE_TRUSTED_PROXIES");
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 0);
}

#[test]
fn dotenv_loading_and_environment_precedence_are_preserved() {
    let sandbox = Sandbox::new();
    let dotenv = sandbox.0.join(".env");
    fs::write(
        &dotenv,
        format!("JWT_SECRET={JWT_SECRET}\nSERVER_PORT={PRIVATE_VALUE}\n"),
    )
    .unwrap();
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env_remove("JWT_SECRET"));
    assert_failure(&output, "SERVER_PORT");
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env_remove("JWT_SECRET")
        .env("SERVER_PORT", "0"));
    assert_success(&output);
    let output = run(sandbox
        .command()
        .arg("--check-config")
        .env("JWT_SECRET", PRIVATE_VALUE)
        .env("SERVER_PORT", "0"));
    assert_failure(&output, "JWT_SECRET");
    fs::write(&dotenv, "JWT_SECRET='unterminated\n").unwrap();
    let output = run(sandbox.command().arg("--check-config"));
    assert_success(&output);
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 1);
}

#[test]
fn configured_xdb_files_must_be_readable_and_match_the_protocol() {
    let sandbox = Sandbox::new();
    write_xdb(&sandbox.0.join("v4.xdb"), 4);
    write_xdb(&sandbox.0.join("v6.xdb"), 6);
    write_xdb(&sandbox.0.join("invalid.xdb"), 42);
    fs::write(sandbox.0.join("truncated.xdb"), [0; 12]).unwrap();
    for key in ["IP_LOCATION_DB_PATH", "IP_LOCATION_V6_DB_PATH"] {
        for path in [PRIVATE_VALUE, "invalid.xdb", "truncated.xdb", "."] {
            let output = run(sandbox.command().arg("--check-config").env(key, path));
            assert_failure(&output, key);
        }
    }
    for (key, path) in [
        ("IP_LOCATION_DB_PATH", "v6.xdb"),
        ("IP_LOCATION_V6_DB_PATH", "v4.xdb"),
    ] {
        let output = run(sandbox.command().arg("--check-config").env(key, path));
        assert_failure(&output, key);
    }
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 4);
}

#[test]
fn unknown_or_extra_arguments_do_not_start_the_service_or_echo_arguments() {
    let sandbox = Sandbox::new();
    for arguments in [vec![PRIVATE_VALUE], vec!["--check-config", PRIVATE_VALUE]] {
        let output = run(sandbox.command().args(arguments));
        assert_failure(&output, "usage: guardian-backend [--check-config]");
    }
    assert_eq!(fs::read_dir(&sandbox.0).unwrap().count(), 0);
}
