# BUNKER MINER - Development Environment Setup

This document provides comprehensive setup instructions for developing BUNKER MINER on Windows 11 and Ubuntu LTS. Following this guide ensures a consistent, secure, and productive development environment.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Core Development Tools](#core-development-tools)
3. [Rust Development Setup](#rust-development-setup)
4. [C++ and Qt Development Setup](#c-development-setup)
5. [GPU Development Kits](#gpu-development-kits)
6. [Security Tools](#security-tools)
7. [IDE Configuration](#ide-configuration)
8. [Project Setup](#project-setup)
9. [Development Workflow](#development-workflow)
10. [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

#### Windows 11
- **OS**: Windows 11 (latest updates)
- **Architecture**: x64 (ARM64 not supported for GPU development)
- **Memory**: 16GB RAM minimum, 32GB recommended
- **Storage**: 50GB free space for development tools and artifacts
- **GPU**: NVIDIA RTX series or AMD RX series (for testing)

#### Ubuntu LTS
- **OS**: Ubuntu 22.04 LTS or 24.04 LTS
- **Architecture**: x86_64
- **Memory**: 16GB RAM minimum, 32GB recommended  
- **Storage**: 50GB free space for development tools and artifacts
- **GPU**: NVIDIA RTX series or AMD RX series (for testing)

### Administrative Access
- **Windows**: Administrator privileges required for driver and tool installation
- **Linux**: `sudo` access required for package installation

## Core Development Tools

### Git Version Control
Version control is mandatory for all development work.

#### Windows Installation
```powershell
# Install Git for Windows
winget install Git.Git

# Or download from: https://git-scm.com/download/win
```

#### Linux Installation
```bash
# Ubuntu/Debian
sudo apt update && sudo apt install -y git

# Configure Git (required)
git config --global user.name "Your Name"
git config --global user.email "your.email@example.com"
```

**Required Git Configuration:**
```bash
# Set default branch name
git config --global init.defaultBranch main

# Enable signing commits (recommended)
git config --global commit.gpgsign true

# Set up line ending handling
git config --global core.autocrlf input  # Linux/macOS
git config --global core.autocrlf true   # Windows
```

### Python 3.9+
Required for development tooling and pre-commit hooks.

#### Windows Installation
```powershell
# Install Python 3.11
winget install Python.Python.3.11

# Verify installation
python --version
pip --version
```

#### Linux Installation
```bash
# Ubuntu 22.04/24.04 comes with Python 3.10/3.11
sudo apt install -y python3 python3-pip python3-venv

# Verify installation
python3 --version
pip3 --version
```

### Docker and Container Tools
Required for local development and testing infrastructure.

#### Windows Installation
```powershell
# Install Docker Desktop
winget install Docker.DockerDesktop

# Enable WSL2 backend for better performance
```

#### Linux Installation
```bash
# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh

# Add user to docker group
sudo usermod -aG docker $USER

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Logout and login to apply group changes
```

## Rust Development Setup

### Rust Toolchain Installation
BUNKER MINER uses Rust for high-performance daemon and pool components.

**Pinned Version**: `1.75.0` (stable)

#### Installation (All Platforms)
```bash
# Install rustup (Rust installer)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Source environment
source $HOME/.cargo/env  # Linux/macOS
# On Windows, restart terminal

# Install specific toolchain version
rustup install 1.75.0
rustup default 1.75.0

# Add required components
rustup component add rustfmt clippy

# Verify installation
rustc --version  # Should show: rustc 1.75.0
cargo --version
```

### Rust Development Tools

#### Essential Cargo Tools
```bash
# Security auditing tool (mandatory)
cargo install cargo-audit

# License checking tool
cargo install cargo-deny

# Code coverage tool
cargo install cargo-tarpaulin

# Cross-compilation support
rustup target add x86_64-pc-windows-gnu   # Linux -> Windows
rustup target add x86_64-unknown-linux-gnu # Windows -> Linux
```

#### IDE Integration
```bash
# Rust Language Server
rustup component add rust-analyzer
```

### Hardware-Specific Rust Dependencies

#### GPU Libraries
```bash
# These will be handled by Cargo, but system libraries are needed:

# NVIDIA CUDA (Windows)
# Download CUDA Toolkit 12.0+ from: https://developer.nvidia.com/cuda-downloads

# AMD ROCm (Linux)
sudo apt install rocm-dev rocm-libs
```

## C++ Development Setup

### C++ Compiler and Build Tools
BUNKER MINER client uses C++17 with Qt6 framework.

**Pinned Versions**:
- **GCC**: 11.x or newer
- **Clang**: 15.x or newer
- **MSVC**: 2022 (17.x) or newer
- **CMake**: 3.20 or newer

#### Windows Setup
```powershell
# Install Visual Studio 2022 Community
winget install Microsoft.VisualStudio.2022.Community

# Install CMake
winget install Kitware.CMake

# Install LLVM (includes clang-format)
winget install LLVM.LLVM
```

**Visual Studio Workloads**:
- Desktop development with C++
- CMake tools for C++
- Git for Windows (if not already installed)

#### Linux Setup
```bash
# Install build tools
sudo apt install -y build-essential cmake ninja-build

# Install Clang tools
sudo apt install -y clang-15 clang-format-15 clang-tidy-15

# Create symlinks for tools
sudo ln -sf /usr/bin/clang-format-15 /usr/bin/clang-format
sudo ln -sf /usr/bin/clang-tidy-15 /usr/bin/clang-tidy

# Install additional development libraries
sudo apt install -y pkg-config libssl-dev
```

### Qt6 Framework
**Pinned Version**: Qt 6.5 LTS

#### Windows Installation
```powershell
# Download Qt Online Installer from: https://www.qt.io/download-open-source-software
# Install Qt 6.5 LTS with:
# - MSVC 2022 64-bit compiler
# - Qt Creator IDE
# - CMake integration
```

#### Linux Installation
```bash
# Install Qt6 development packages
sudo apt install -y qt6-base-dev qt6-tools-dev qt6-tools-dev-tools

# Install additional Qt6 modules
sudo apt install -y qt6-base-private-dev libqt6core5compat6-dev

# Verify installation
qmake6 --version
```

### Protocol Buffers and gRPC
**Pinned Versions**:
- **Protocol Buffers**: 3.21.x
- **gRPC**: 1.50.x

#### Windows Installation
```powershell
# Install via vcpkg (recommended)
git clone https://github.com/Microsoft/vcpkg.git C:\vcpkg
cd C:\vcpkg
.\bootstrap-vcpkg.bat
.\vcpkg integrate install

# Install gRPC and protobuf
.\vcpkg install grpc:x64-windows protobuf:x64-windows
```

#### Linux Installation
```bash
# Install Protocol Buffers
sudo apt install -y protobuf-compiler libprotobuf-dev

# Install gRPC
sudo apt install -y libgrpc++-dev libgrpc-dev protobuf-compiler-grpc
```

## GPU Development Kits

### NVIDIA CUDA Toolkit
**Pinned Version**: CUDA 12.0 or newer

#### Windows Installation
1. Download CUDA Toolkit from [NVIDIA Developer](https://developer.nvidia.com/cuda-downloads)
2. Choose Windows x86_64 → Network Installer
3. Install with default settings
4. Verify installation:
   ```cmd
   nvcc --version
   nvidia-smi
   ```

#### Linux Installation
```bash
# Ubuntu 22.04/24.04
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.0-1_all.deb
sudo dpkg -i cuda-keyring_1.0-1_all.deb
sudo apt-get update
sudo apt-get -y install cuda

# Verify installation
nvcc --version
nvidia-smi
```

### AMD ROCm Platform
**Pinned Version**: ROCm 5.4 or newer

#### Linux Installation (AMD GPUs)
```bash
# Ubuntu 22.04/24.04
wget -q -O - https://repo.radeon.com/rocm/rocm.gpg.key | sudo apt-key add -
echo 'deb [arch=amd64] https://repo.radeon.com/rocm/apt/5.4/ ubuntu main' | sudo tee /etc/apt/sources.list.d/rocm.list
sudo apt update
sudo apt install -y rocm-dev rocm-libs rocm-utils

# Add user to render group
sudo usermod -aG render $USER
sudo usermod -aG video $USER

# Verify installation
rocminfo
```

**Note**: ROCm on Windows is not officially supported. AMD GPU development on Windows uses proprietary APIs.

## Security Tools

### Mandatory Security Tools
All developers must install and configure these security tools.

#### Pre-commit Hooks
```bash
# Install pre-commit
pip install pre-commit

# Setup will be done during project setup
```

#### Cargo Audit (Rust Security)
```bash
# Already installed in Rust setup above
cargo audit --version
```

#### Static Analysis Tools

##### Rust Security
```bash
# Additional security tools
cargo install cargo-geiger    # Detect unsafe code usage
cargo install cargo-outdated  # Check for outdated dependencies
```

##### C++ Security
```bash
# Linux
sudo apt install -y cppcheck

# Windows (via Chocolatey)
choco install cppcheck
```

### IDE Security Plugins

#### Visual Studio Code Extensions
```bash
# Install VS Code security extensions
code --install-extension rust-lang.rust-analyzer
code --install-extension ms-vscode.cpptools
code --install-extension ms-vscode.cmake-tools
code --install-extension esbenp.prettier-vscode
```

#### CLion/IntelliJ Security Plugins
- **SonarLint**: Real-time security analysis
- **Checkmarx**: Static security analysis

## IDE Configuration

### Visual Studio Code (Recommended)
Create `.vscode/settings.json` in project root:

```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.checkOnSave.extraArgs": ["--", "-D", "warnings"],
    "C_Cpp.cppStandard": "c++17",
    "C_Cpp.intelliSenseEngine": "Default",
    "cmake.configureOnOpen": true,
    "files.associations": {
        "*.toml": "toml",
        "*.proto": "proto3"
    },
    "editor.formatOnSave": true,
    "editor.codeActionsOnSave": {
        "source.fixAll": true
    }
}
```

### CLion Configuration
1. **Rust Plugin**: Install Rust plugin from JetBrains Marketplace
2. **CMake Profile**: Configure CMake profiles for Debug/Release
3. **Code Style**: Import project code style settings
4. **External Tools**: Configure cargo commands

## Project Setup

### Initial Repository Setup

1. **Clone the Repository**:
   ```bash
   git clone https://github.com/emiliancristea/bunker-miner.git
   cd bunker-miner
   ```

2. **Setup Pre-commit Hooks**:
   ```bash
   # Run the setup script
   ./tools/setup-pre-commit.sh
   ```

3. **Verify Rust Build**:
   ```bash
   # Test daemon build
   cd daemon
   cargo build
   cargo test
   cd ..
   
   # Test pool build  
   cd pool
   cargo build
   cargo test
   cd ..
   
   # Test shared library
   cd libs/common-rust
   cargo build
   cargo test
   cd ../..
   ```

4. **Verify C++ Build** (requires Qt6 and CMake):
   ```bash
   cd client
   mkdir build && cd build
   cmake ..
   cmake --build .
   cd ../..
   ```

5. **Setup Development Environment**:
   ```bash
   # Create local configuration
   cp .env.example .env  # When available
   
   # Start development services
   docker-compose up -d  # When available
   ```

### Environment Variables

Create a `.env` file in the project root (this file is gitignored):

```bash
# Development Environment Variables

# Rust
RUST_LOG=debug
RUST_BACKTRACE=1

# C++ Build
CMAKE_BUILD_TYPE=Debug

# Development Database URLs
DATABASE_URL=postgresql://postgres:password@localhost:5432/bunker_dev
REDIS_URL=redis://localhost:6379

# API Keys (use dummy values for development)
COINGECKO_API_KEY=your_api_key_here
POOL_API_KEY=dev_api_key

# Mining Configuration (test values only)
TEST_WALLET_BTC=bc1qtest...
TEST_POOL_URL=stratum+tcp://testpool.com:4444
```

## Development Workflow

### Daily Development Process

1. **Update Dependencies**:
   ```bash
   git pull origin develop
   cargo update            # Update Rust dependencies
   pre-commit autoupdate  # Update pre-commit hooks
   ```

2. **Create Feature Branch**:
   ```bash
   git checkout develop
   git checkout -b feature/your-feature-name
   ```

3. **Development Cycle**:
   ```bash
   # Make changes
   # Run tests frequently
   cargo test              # Rust tests
   cmake --build build     # C++ build
   
   # Run security checks
   cargo audit
   pre-commit run --all-files
   ```

4. **Commit Changes**:
   ```bash
   git add .
   git commit -m "feat: your change description"
   # Pre-commit hooks run automatically
   ```

5. **Push and Create PR**:
   ```bash
   git push origin feature/your-feature-name
   # Create pull request via GitHub
   ```

### Code Quality Standards

#### Rust Code Standards
- **Formatting**: Enforced by `rustfmt`
- **Linting**: Enforced by `clippy` with warnings as errors
- **Testing**: Minimum 80% code coverage
- **Documentation**: All public APIs must be documented

#### C++ Code Standards  
- **Standard**: C++17
- **Formatting**: Google style via `clang-format`
- **Naming**: CamelCase for classes, snake_case for functions
- **Memory**: Prefer smart pointers, avoid raw pointers

#### Security Standards
- **No Hardcoded Secrets**: Enforced by pre-commit hooks
- **Input Validation**: All external inputs must be validated
- **Error Handling**: Proper error handling for all operations
- **Audit Trail**: Security-relevant actions must be logged

## Troubleshooting

### Common Build Issues

#### Rust Build Problems
```bash
# Clear build cache
cargo clean
rm -rf target/

# Update toolchain
rustup update

# Check for missing system libraries
ldd target/debug/bunker-miner-daemon  # Linux
```

#### C++ Build Problems
```bash
# Clear CMake cache
rm -rf build/
mkdir build && cd build

# Regenerate build files
cmake .. -DCMAKE_BUILD_TYPE=Debug

# Check Qt6 installation
qmake6 --version
```

#### GPU Development Issues

##### NVIDIA CUDA
```bash
# Check driver compatibility
nvidia-smi
nvcc --version

# Test CUDA installation
cd /usr/local/cuda/samples/1_Utilities/deviceQuery
sudo make
./deviceQuery
```

##### AMD ROCm
```bash
# Check ROCm installation
rocminfo
rocm-smi

# Check permissions
ls -la /dev/kfd
```

### Common Development Environment Issues

#### Pre-commit Hook Failures
```bash
# Skip hooks temporarily (not recommended)
git commit --no-verify -m "emergency commit"

# Fix formatting issues
cargo fmt
clang-format -i client/src/*.cpp

# Update pre-commit
pre-commit autoupdate
```

#### Permission Issues (Linux)
```bash
# Docker permission denied
sudo usermod -aG docker $USER
# Logout and login again

# GPU access permission
sudo usermod -aG video $USER
sudo usermod -aG render $USER
```

#### Missing Dependencies
```bash
# Ubuntu: Fix broken dependencies
sudo apt update && sudo apt -f install

# Missing protobuf compiler
sudo apt install protobuf-compiler

# Missing Qt6 development headers
sudo apt install qt6-base-dev qt6-tools-dev
```

### Getting Help

#### Internal Resources
1. **Documentation**: Check `docs/` directory
2. **Issues**: Search GitHub issues
3. **Development Chat**: Internal team communication channels

#### External Resources
1. **Rust**: [The Rust Book](https://doc.rust-lang.org/book/)
2. **Qt6**: [Qt Documentation](https://doc.qt.io/)
3. **CMake**: [CMake Documentation](https://cmake.org/documentation/)
4. **GPU Development**: NVIDIA/AMD developer documentation

#### Reporting Issues
When reporting development environment issues, include:

1. **System Information**:
   ```bash
   # System details
   uname -a                    # Linux
   systeminfo                 # Windows
   
   # Tool versions
   rustc --version
   cmake --version
   qmake6 --version
   ```

2. **Error Messages**: Complete error output
3. **Steps to Reproduce**: Exact commands that cause the issue
4. **Environment Variables**: Relevant environment settings

---

## Security Considerations

### Development Security Best Practices

1. **Never commit secrets**: Pre-commit hooks prevent this, but remain vigilant
2. **Use test data**: Never use real wallet addresses or API keys in development
3. **Keep tools updated**: Regularly update all development tools for security patches
4. **Secure development**: Use encrypted storage for development machines
5. **Network security**: Use VPN for accessing internal development resources

### Required Security Training

All developers must complete the initial security awareness training covering:

- **OWASP Top 10**: Web application security risks
- **Memory Safety**: C++ memory management best practices  
- **Rust Safety**: Understanding `unsafe` blocks and when to use them
- **Cryptographic Mining Risks**: Malicious miners and RPC endpoint security
- **Supply Chain Security**: Dependency verification and management

Training materials and certification tracking are maintained separately from this document.

---

*This development environment guide is part of the BUNKER MINER project governance framework. It must be kept up-to-date as tools and requirements evolve.*

**Last Updated**: Phase 0.1 - Initial Version  
**Next Review**: Phase 0.2 (Technology PoC Validation)