# Testing

Crosvm runs on a variety of platforms with a significant amount of platform-specific code. Testing
on all the supported platforms is crucial to keep crosvm healthy.

## Types of tests

### Unit Tests

Unit tests are your standard rust tests embedded with the rest of the code in `src/` and wrapped in
a `#[cfg(test)]` attribute.

Unit tests **cannot make any guarantees on the runtime environment**. Avoid doing the following in
unit tests:

- Avoid kernel features such as io_uring or userfaultfd, which may not be available on all kernels.
- Avoid functionality that requires privileges (e.g. CAP_NET_ADMIN)
- Avoid spawning threads or processes
- Avoid accessing kernel devices
- Avoid global state in unit tests

This allows us to execute unit tests for any platform using emulators such as qemu-user-static or
wine64.

#### File Access in Unit Tests

Some unit tests may need to access extra data files. The files should be accessed at build time
using `include_str!()` macro, rather than at run time. The file is located relative to the current
file (similarly to how modules are found). The contents of the file can be used directly in the test
or at runtime the test can write this data to a temporary file. This approach is crucial because
certain test environment may require to run the test binaries directly without access to the source
code. Additionally, it ensures the test binary can be run manually within a debugger like GDB.

These approaches ensure that units tests be able to find the correct paths in various build &
execution environment.

**Example:**

```rust
#[test]
fn test_my_config() {
    let temp_file = TempDir::new().unwrap();
    let path = temp_file.path().join("my_config.cfg");
    let test_config = include_str!("../../../data/config/my_config.cfg");
    fs::write(&path, test_config).expect("Unable to write test file");
    let config_file = File::open(path).expect("Failed to open config file");
    // ... rest of your test ...
}
```

### Documentation tests

Rust's
[documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html)
can be used to provide examples as part of the documentation that is verified by CI.

Documentation tests are slow and not run as part of the usual workflows, but can be run locally
with:

```sh
./tools/presubmit doc_tests
```

### Integration tests

Cargo has native support for
[integration testing](https://doc.rust-lang.org/rust-by-example/testing/integration_testing.html).
Integration tests are written just like unit tests, but live in a separate directory at `tests/`.

Integration tests **guarantee that the test has privileged access to the test environment**. They
are only executed when a device-under-test (DUT) is specified when running tests:

```sh
./tools/run_tests --dut=vm|host
```

### End To End (E2E) tests

End to end tests live in the `e2e_tests` crate. The crate provides a framework to boot a guest with
crosvm and execut commands in the guest to validate functionality at a high level.

E2E tests are executed just like integration tests. By giving
[nextest's filter expressions](https://nexte.st/book/filter-expressions), you can run a subset of
the tests.

```sh
# Run all e2e tests
./tools/run_tests --dut=vm --filter-expr 'package(e2e_tests)'
# Run e2e tests whose name contains the string 'boot'.
./tools/run_tests --dut=vm --filter-expr 'package(e2e_tests) and test(boot)'
```

### Downstream Product tests

Each downstream product that uses crosvm is performing their own testing, e.g. ChromeOS is running
high level testing of its VM features on ChromeOS hardware, while AOSP is running testing of their
VM features on AOSP hardware.

Upstream crosvm is not involved in these tests and they are not executed in crosvm CI.

## Parallel test execution

Crosvm tests are executed in parallel, each test case in its own process via
[cargo nextest](http://nexte.st).

This requires tests to be cautious about global state, especially integration tests which interact
with system devices.

If you require exclusive access to a device or file, you have to use
[file-based locking](https://docs.rs/named-lock/latest/named_lock) to prevent access by other test
processes.

## Platforms tested

The platforms below can all be tested using `tools/run_tests -p $platform`. The table indicates how
these tests are executed:

| Platform                    | Build |         Unit Tests         | Integration Tests | E2E Tests |
| :-------------------------- | :---: | :------------------------: | :---------------: | :-------: |
| x86_64 (linux)              |  ✅   |             ✅             |        ✅         |    ✅     |
| aarch64 (linux)             |  ✅   | ✅ (qemu-user[^qemu-user]) | ✅ (qemu[^qemu])  |    ❌     |
| armhf (linux)               |  ✅   | ✅ (qemu-user[^qemu-user]) |        ❌         |    ❌     |
| mingw64[^windows] (linux)   |  🚧   |        🚧 (wine64)         |        ❌         |    ❌     |
| mingw64[^windows] (windows) |  🚧   |             🚧             |        🚧         |    ❌     |

Crosvm CI will use the same configuration as `tools/run_tests`.

## Debugging Tips

Here are some tips for developing or/and debugging crosvm tests.

### Enter a test VM to see logs

When you run a test on a VM with `./tools/run_tests --dut=vm`, if the test fails, you'll see
extracted log messages. To see the full messages or monitor the test process during the runtime, you
may want to enter the test VM.

First, enter the VM's shell and start printing the syslog:

```console
$ ./tools/dev_container # Enter the dev_container
$ ./tools/x86vm shell   # Enter the test VM
crosvm@testvm-x8664:~$ journalctl -f
# syslog messages will be printed...
```

Then, open another terminal and run a test:

```console
$ ./tools/run_tests --dut=vm --filter-expr 'package(e2e_tests) and test(boot)'
```

So you'll see the crosvm log in the first terminal.

[^qemu-user]: qemu-aarch64-static or qemu-arm-static translate instructions into x86 and executes them on the
    host kernel. This works well for unit tests, but will fail when interacting with platform
    specific kernel features.

[^qemu]: run_tests will launch a VM for testing in the background. This VM is using full system
    emulation, which causes tests to be slow. Also not all aarch64 features are properly emulated,
    which prevents us from running e2e tests.

[^windows]: Windows builds of crosvm are a work in progress. Some tests are executed via wine64 on linux
