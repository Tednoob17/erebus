//! Seccomp BPF filter for the erebus launcher stub.
//!
//! Provides `install_seccomp_filter()` which installs a capability-driven BPF
//! filter after `pivot_root`. Linux-only.
//!
//! The filter is built from the artifact's declared capabilities:
//! - `Exec` → allow `execve`, `execveat` (needed for entrypoint)
//! - `Network` → allow socket/syscalls for networking
//! - `Syscall` → allow dangerous syscalls (mount, `pivot_root`, etc.)
//! - Without a capability, the corresponding syscalls are denied.
//!
//! Apps that work without seccomp continue working with it.

#[cfg(target_os = "linux")]
use std::io;

#[cfg(target_os = "linux")]
mod arch_consts {
    #[cfg(target_arch = "x86_64")]
    pub const AUDIT_ARCH: u32 = 0xC000_003E;
    #[cfg(target_arch = "aarch64")]
    pub const AUDIT_ARCH: u32 = 0xC000_00B7;
    #[cfg(target_arch = "x86")]
    pub const AUDIT_ARCH: u32 = 0x4000_0003;
    #[cfg(target_arch = "arm")]
    pub const AUDIT_ARCH: u32 = 0x4000_0028;
}

#[cfg(target_os = "linux")]
use arch_consts::AUDIT_ARCH;

use erebus_core::layer::Capability;

/// Capability-driven seccomp filter configuration.
#[derive(Debug, Clone, Default)]
pub struct SeccompConfig {
    /// Allow `execve`, `execveat` (entrypoint execution).
    pub allow_exec: bool,
    /// Allow networking syscalls (connect, bind, listen, etc.).
    pub allow_network: bool,
    /// Allow dangerous syscalls (mount, `pivot_root`, kexec, etc.).
    pub allow_syscalls: bool,
}

impl SeccompConfig {
    /// Build config from a set of capabilities.
    pub fn from_capabilities(caps: &[Capability]) -> Self {
        let mut cfg = Self::default();
        for cap in caps {
            match cap {
                Capability::Exec => cfg.allow_exec = true,
                Capability::Network => cfg.allow_network = true,
                Capability::Syscall => cfg.allow_syscalls = true,
                _ => {} // ReadFile, WriteFile, Env don't affect seccomp directly
            }
        }
        cfg
    }
}

/// Install a capability-driven seccomp-bpf filter after `pivot_root`. Linux-only.
#[cfg(target_os = "linux")]
pub fn install_seccomp_filter(config: &SeccompConfig) -> io::Result<()> {
    let filter = build_seccomp_filter(config);
    let prog = libc::sock_fprog {
        len: filter.len() as u16,
        filter: filter.as_ptr().cast_mut(),
    };

    // SAFETY: prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog) installs
    // a BPF filter on the current process. The filter program and its data
    // are stack-allocated Vec that outlive the prctl call. The *mut cast on
    // filter is required by sock_fprog's C ABI; prctl only reads the filter
    // (no mutation through the pointer). On success the filter is permanent
    // — any blocked syscall kills the process with SIGSYS.
    let rc = unsafe {
        libc::prctl(
            libc::PR_SET_SECCOMP,
            2,
            std::ptr::from_ref(&prog) as usize,
            0,
            0,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Build the seccomp BPF filter program for the current architecture.
///
/// The filter is a denylist by default, but allows syscalls required by
/// the declared capabilities. Everything else falls through to ALLOW.
#[cfg(target_os = "linux")]
pub(crate) fn build_seccomp_filter(config: &SeccompConfig) -> Vec<libc::sock_filter> {
    // BPF instruction encodings (linux/filter.h).
    const BPF_LD: u16 = 0x00;
    const BPF_W: u16 = 0x00;
    const BPF_ABS: u16 = 0x20;
    const BPF_JMP: u16 = 0x05;
    const BPF_JEQ: u16 = 0x10;
    const BPF_RET: u16 = 0x06;
    const BPF_K: u16 = 0x00;

    // kexec_file_load is x86_64-only; other arches reuse kexec_load.
    #[cfg(target_arch = "x86_64")]
    const SYS_KEXEC_FILE_LOAD: u32 = libc::SYS_kexec_file_load as u32;
    #[cfg(not(target_arch = "x86_64"))]
    const SYS_KEXEC_FILE_LOAD: u32 = libc::SYS_kexec_load as u32;

    const SYS_PTRACE: u32 = libc::SYS_ptrace as u32;
    const SYS_BPF: u32 = libc::SYS_bpf as u32;
    const SYS_USERFAULTFD: u32 = libc::SYS_userfaultfd as u32;
    const SYS_MOUNT: u32 = libc::SYS_mount as u32;
    const SYS_UMOUNT2: u32 = libc::SYS_umount2 as u32;
    const SYS_PIVOT_ROOT: u32 = libc::SYS_pivot_root as u32;
    const SYS_REBOOT: u32 = libc::SYS_reboot as u32;
    const SYS_SETHOSTNAME: u32 = libc::SYS_sethostname as u32;
    const SYS_SETDOMAINNAME: u32 = libc::SYS_setdomainname as u32;
    const SYS_SWAPON: u32 = libc::SYS_swapon as u32;
    const SYS_SWAPOFF: u32 = libc::SYS_swapoff as u32;
    const SYS_ACCT: u32 = libc::SYS_acct as u32;
    const SYS_KEXEC_LOAD: u32 = libc::SYS_kexec_load as u32;
    const SYS_INIT_MODULE: u32 = libc::SYS_init_module as u32;
    const SYS_FINIT_MODULE: u32 = libc::SYS_finit_module as u32;
    const SYS_DELETE_MODULE: u32 = libc::SYS_delete_module as u32;
    const SYS_NFSSERVCTL: u32 = libc::SYS_nfsservctl as u32;

    // Networking syscalls
    const SYS_SOCKET: u32 = libc::SYS_socket as u32;
    const SYS_CONNECT: u32 = libc::SYS_connect as u32;
    const SYS_BIND: u32 = libc::SYS_bind as u32;
    const SYS_LISTEN: u32 = libc::SYS_listen as u32;
    const SYS_ACCEPT: u32 = libc::SYS_accept as u32;
    const SYS_ACCEPT4: u32 = libc::SYS_accept4 as u32;
    const SYS_SENDTO: u32 = libc::SYS_sendto as u32;
    const SYS_RECVFROM: u32 = libc::SYS_recvfrom as u32;
    const SYS_SENDMSG: u32 = libc::SYS_sendmsg as u32;
    const SYS_RECVMSG: u32 = libc::SYS_recvmsg as u32;

    // Exec syscalls
    const SYS_EXECVE: u32 = libc::SYS_execve as u32;
    const SYS_EXECVEAT: u32 = libc::SYS_execveat as u32;

    // Seccomp return codes — use libc constants (stable kernel ABI values).
    const SECCOMP_RET_KILL_PROCESS: u32 = libc::SECCOMP_RET_KILL_PROCESS;
    const SECCOMP_RET_ALLOW: u32 = libc::SECCOMP_RET_ALLOW;

    let arch_load = |code: u16, jt: u8, jf: u8, k: u32| libc::sock_filter {
        code: code | BPF_W | BPF_ABS,
        jt,
        jf,
        k,
    };
    let jmp_eq = |k: u32, jt: u8, jf: u8| libc::sock_filter {
        code: BPF_JMP | BPF_JEQ | BPF_K,
        jt,
        jf,
        k,
    };
    let ret = |k: u32| libc::sock_filter {
        code: BPF_RET | BPF_K,
        jt: 0,
        jf: 0,
        k,
    };

    // Build the filter dynamically based on capabilities.
    // Start with base: arch check + syscall number load
    let mut filter = vec![
        arch_load(BPF_LD, 0, 0, 4), //  0: load architecture
        jmp_eq(AUDIT_ARCH, 0, 3),   //  1: arch mismatch -> skip to syscall load (ALLOW)
        arch_load(BPF_LD, 0, 0, 0), //  2: load syscall number (after arch check)
    ];

    // If Exec capability NOT granted, deny execve/execveat
    if !config.allow_exec {
        filter.push(jmp_eq(SYS_EXECVE, 0, 1)); // match -> deny
        filter.push(jmp_eq(SYS_EXECVEAT, 0, 1)); // match -> deny
        filter.push(ret(SECCOMP_RET_KILL_PROCESS)); // deny target
    }

    // If Network capability NOT granted, deny networking syscalls
    if !config.allow_network {
        filter.push(jmp_eq(SYS_SOCKET, 0, 1));
        filter.push(jmp_eq(SYS_CONNECT, 0, 1));
        filter.push(jmp_eq(SYS_BIND, 0, 1));
        filter.push(jmp_eq(SYS_LISTEN, 0, 1));
        filter.push(jmp_eq(SYS_ACCEPT, 0, 1));
        filter.push(jmp_eq(SYS_ACCEPT4, 0, 1));
        filter.push(jmp_eq(SYS_SENDTO, 0, 1));
        filter.push(jmp_eq(SYS_RECVFROM, 0, 1));
        filter.push(jmp_eq(SYS_SENDMSG, 0, 1));
        filter.push(jmp_eq(SYS_RECVMSG, 0, 1));
        filter.push(ret(SECCOMP_RET_KILL_PROCESS));
    }

    // If Syscall capability NOT granted, deny dangerous syscalls
    if !config.allow_syscalls {
        filter.push(jmp_eq(SYS_PTRACE, 0, 1));
        filter.push(jmp_eq(SYS_BPF, 0, 1));
        filter.push(jmp_eq(SYS_USERFAULTFD, 0, 1));
        filter.push(jmp_eq(SYS_MOUNT, 0, 1));
        filter.push(jmp_eq(SYS_UMOUNT2, 0, 1));
        filter.push(jmp_eq(SYS_PIVOT_ROOT, 0, 1));
        filter.push(jmp_eq(SYS_REBOOT, 0, 1));
        filter.push(jmp_eq(SYS_SETHOSTNAME, 0, 1));
        filter.push(jmp_eq(SYS_SETDOMAINNAME, 0, 1));
        filter.push(jmp_eq(SYS_SWAPON, 0, 1));
        filter.push(jmp_eq(SYS_SWAPOFF, 0, 1));
        filter.push(jmp_eq(SYS_ACCT, 0, 1));
        filter.push(jmp_eq(SYS_KEXEC_LOAD, 0, 1));
        filter.push(jmp_eq(SYS_INIT_MODULE, 0, 1));
        filter.push(jmp_eq(SYS_FINIT_MODULE, 0, 1));
        filter.push(jmp_eq(SYS_DELETE_MODULE, 0, 1));
        filter.push(jmp_eq(SYS_NFSSERVCTL, 0, 1));
        filter.push(jmp_eq(SYS_KEXEC_FILE_LOAD, 0, 1));
        filter.push(ret(SECCOMP_RET_KILL_PROCESS));
    }

    // Fallthrough: ALLOW everything else
    filter.push(ret(SECCOMP_RET_ALLOW));

    filter
}

/// Backward compatibility: install the classic hardcoded denylist.
#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub fn install_seccomp_denylist() -> io::Result<()> {
    install_seccomp_filter(&SeccompConfig {
        allow_exec: true,
        allow_network: true,
        allow_syscalls: false,
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use erebus_core::layer::Capability;

    /// BPF opcode masks for validation.
    const BPF_RET: u16 = 0x06;
    const BPF_K: u16 = 0x00;

    #[test]
    fn validate_default_config_allows_exec_network_denies_syscalls() {
        let filter = build_seccomp_filter(&SeccompConfig {
            allow_exec: true,
            allow_network: true,
            allow_syscalls: false,
        });

        // Should NOT have deny rules for exec/network, but SHOULD for syscalls
        // Final instruction must be ALLOW
        let last = filter.last().unwrap();
        assert_eq!(last.code, BPF_RET | BPF_K);
        assert_eq!(last.k, libc::SECCOMP_RET_ALLOW);

        // Check that execve is NOT denied (allow_exec = true)
        let execve_denied = filter.iter().enumerate().skip(3).any(|(_, insn)| {
            if insn.code & 0x05 == 0x05 {
                insn.k == libc::SYS_execve as u32
            } else {
                false
            }
        });
        assert!(
            !execve_denied,
            "execve should be allowed when allow_exec=true"
        );
    }

    #[test]
    fn validate_no_capabilities_denies_all() {
        let filter = build_seccomp_filter(&SeccompConfig::default());

        // Should deny exec, network, and syscalls
        // Final instruction must be ALLOW
        let last = filter.last().unwrap();
        assert_eq!(last.code, BPF_RET | BPF_K);
        assert_eq!(last.k, libc::SECCOMP_RET_ALLOW);

        // Check that execve IS denied (allow_exec = false)
        let execve_denied = filter.iter().enumerate().skip(3).any(|(_, insn)| {
            if insn.code & 0x05 == 0x05 {
                insn.k == libc::SYS_execve as u32
            } else {
                false
            }
        });
        assert!(
            execve_denied,
            "execve should be denied when allow_exec=false"
        );

        // Check that connect IS denied (allow_network = false)
        let connect_denied = filter.iter().enumerate().skip(3).any(|(_, insn)| {
            if insn.code & 0x05 == 0x05 {
                insn.k == libc::SYS_connect as u32
            } else {
                false
            }
        });
        assert!(
            connect_denied,
            "connect should be denied when allow_network=false"
        );

        // Check that ptrace IS denied (allow_syscalls = false)
        let ptrace_denied = filter.iter().enumerate().skip(3).any(|(_, insn)| {
            if insn.code & 0x05 == 0x05 {
                insn.k == libc::SYS_ptrace as u32
            } else {
                false
            }
        });
        assert!(
            ptrace_denied,
            "ptrace should be denied when allow_syscalls=false"
        );
    }

    #[test]
    fn validate_capability_config_mapping() {
        let config = SeccompConfig::from_capabilities(&[Capability::Exec]);
        assert!(config.allow_exec);
        assert!(!config.allow_network);
        assert!(!config.allow_syscalls);

        let config = SeccompConfig::from_capabilities(&[Capability::Network]);
        assert!(!config.allow_exec);
        assert!(config.allow_network);
        assert!(!config.allow_syscalls);

        let config = SeccompConfig::from_capabilities(&[Capability::Syscall]);
        assert!(!config.allow_exec);
        assert!(!config.allow_network);
        assert!(config.allow_syscalls);

        let config = SeccompConfig::from_capabilities(&[
            Capability::Exec,
            Capability::Network,
            Capability::Syscall,
        ]);
        assert!(config.allow_exec);
        assert!(config.allow_network);
        assert!(config.allow_syscalls);

        // ReadFile, WriteFile, Env don't affect seccomp
        let config = SeccompConfig::from_capabilities(&[
            Capability::ReadFile,
            Capability::WriteFile,
            Capability::Env,
        ]);
        assert!(!config.allow_exec);
        assert!(!config.allow_network);
        assert!(!config.allow_syscalls);
    }

    #[test]
    fn validate_seccomp_filter_is_denylist_not_denyall() {
        let filter = build_seccomp_filter(&SeccompConfig {
            allow_exec: true,
            allow_network: true,
            allow_syscalls: false,
        });

        // THE critical "not deny-all" invariant: the final fallthrough return
        // must be ALLOW, not KILL_PROCESS. If this ever flips, every syscall
        // is blocked and the launcher can no longer exec anything.
        let allow = filter.last().unwrap();
        assert_eq!(allow.code, BPF_RET | BPF_K);
        assert_eq!(
            allow.k,
            libc::SECCOMP_RET_ALLOW,
            "last instruction must fall through to ALLOW (denylist, not deny-all)"
        );

        // The deny target (reached on a matched dangerous syscall) must actually
        // kill: KILL_PROCESS is 0x8000_0000 (non-zero, not a silent no-op).
        let kill = &filter[filter.len() - 2];
        assert_eq!(kill.code, BPF_RET | BPF_K);
        assert_eq!(kill.k, libc::SECCOMP_RET_KILL_PROCESS);
        assert_ne!(kill.k, 0, "KILL target must be a real kill, not a no-op");

        // BPF program length fits in the u16 `len` field sock_fprog uses.
        assert!(u16::try_from(filter.len()).is_ok());
    }
}
