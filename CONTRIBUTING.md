# Contributing to VER

Thank you for your interest in contributing to **VER (Very Easy Remote Manager)**! We welcome contributions from everyone, whether it's reporting a bug, improving documentation, suggesting features, or submitting code changes.

---

## Code of Conduct

All contributors are expected to uphold our [Code of Conduct](CODE_OF_CONDUCT.md). Please read it before participating.

---

## How Can I Contribute?

### 1. Reporting Bugs

Before creating a bug report, please check existing issues to see if the problem has already been reported.

When filing a bug report, please include:
* A clear and descriptive title.
* Steps to reproduce the issue.
* Expected vs. actual behavior.
* Your operating system, desktop environment (GNOME, KDE, Wayland/X11), and VER version (`ver --version` or from About dialog).
* Relevant terminal or error logs (sanitize any passwords or sensitive IP addresses).

### 2. Suggesting Enhancements

Feature requests are very welcome! Please open an issue outlining:
* What problem the feature solves.
* A clear description of the proposed functionality or UI design.
* Any alternative solutions or workarounds considered.

### 3. Submitting Pull Requests

1. **Fork and Clone**:
   ```bash
   git clone https://github.com/dawiisss/ver.git
   cd ver
   ```

2. **Create a Feature Branch**:
   ```bash
   git checkout -b feature/my-new-feature
   ```

3. **Install Dependencies**:
   * Refer to the build dependencies listed in [README.md](README.md#build-dependencies).

4. **Develop and Test**:
   Make sure all tests pass and code is formatted cleanly:
   ```bash
   # Run all unit and integration tests
   cargo test --all-targets

   # Check linter warnings
   cargo clippy --all-targets --all-features -- -D warnings

   # Format your code
   cargo fmt --all -- --check
   ```

5. **Commit Conventions**:
   Follow standard [Conventional Commits](https://www.conventionalcommits.org/):
   * `feat: add support for SSH agent forwarding`
   * `fix: handle reconnection on closed socket`
   * `docs: update packaging guide`
   * `style: apply cargo fmt`

6. **Submit PR**:
   Push your branch and open a Pull Request against `main`. Fill in the PR template with relevant context and verification steps.

---

## Architecture & Code Guidelines

* **Rust / GTK4 / Libadwaita**: Maintain GNOME Human Interface Guidelines (HIG) compliance.
* **Concurrency**: Use non-blocking async tasks and channels (`async_channel`, `glib::MainContext`) to keep the UI smooth and responsive. Never block the GTK main thread with network or disk I/O.
* **Security First**: Passwords must always be stored via Secret Service Keyring (`oo7`) or piped via stdin (`/from-stdin:force`). Never write unencrypted credentials to configuration files or logs.
