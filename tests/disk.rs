// Take a look at the license at the top of the repository in the LICENSE file.

#[test]
#[cfg(all(feature = "system", feature = "disk"))]
fn test_disks() {
    if sysinfo::IS_SUPPORTED_SYSTEM {
        let s = sysinfo::System::new_all();
        // If we don't have any physical core present, it's very likely that we're inside a VM...
        if s.physical_core_count().unwrap_or_default() > 0 {
            let mut disks = sysinfo::Disks::new();
            assert!(disks.list().is_empty());
            disks.refresh_list();
            assert!(!disks.list().is_empty());
        }
    }
}

#[test]
#[cfg(feature = "disk")]
fn test_disks_usage() {
    use std::{
        io::Write,
        path::{Path, PathBuf},
    };

    let s = sysinfo::System::new_all();
    dbg!(s.physical_core_count());

    if !sysinfo::IS_SUPPORTED_SYSTEM || s.physical_core_count().unwrap_or_default() == 0 {
        return;
    }

    let mut disks = sysinfo::Disks::new_with_refreshed_list();
    for disk in disks.list() {
        println!("BEFORE: {disk:?}");
    }

    let diskstats = std::fs::read_to_string("/proc/diskstats").unwrap();
    println!("PROCSTATS BEFORE:\n{}", diskstats);

    let mut temp_dir = PathBuf::from(&env!("CARGO_TARGET_TMPDIR"));
    dbg!(&temp_dir);
    temp_dir.pop();
    let path = temp_dir.join("disk_usage_test.tmp");
    dbg!(&path);
    let mut file = std::fs::File::create(path).unwrap();

    // Write 10mb worth of data to the temp file. Repeat this 10 times to increase the chances
    // of the OS registering the disk writes.
    for _ in 0..10 {
        let data = vec![1u8; 10 * 1024 * 1024];
        file.write_all(&data).unwrap();
        // The sync_all call is important to ensure all the data is persisted to disk. Without
        // the call, this test is flaky.
        file.sync_all().unwrap();
    }

    std::thread::sleep(std::time::Duration::from_secs(5));

    disks.refresh();

    let diskstats_after = std::fs::read_to_string("/proc/diskstats").unwrap();

    println!("PROCSTATS AFTER:\n{}", diskstats_after);

    for disk in disks.list() {
        println!("AFTER: {disk:?}");
    }
    // Depending on the OS and how disks are configured, the disk usage may be the exact same
    // across multiple disks. To account for this, collect the disk usages and dedup
    let mut disk_usages = disks.list().iter().map(|d| d.usage()).collect::<Vec<_>>();
    disk_usages.dedup();

    let mut written_bytes = 0;
    for disk_usage in disk_usages {
        written_bytes += disk_usage.written_bytes;
    }

    // written_bytes should have increased by about 10mb, but this is not fully reliable in CI Linux. For now,
    // just verify the number is non-zero.
    #[cfg(not(target_os = "freebsd"))]
    assert!(written_bytes > 0);
    // Disk usage is not yet supported on freebsd
    #[cfg(target_os = "freebsd")]
    assert_eq!(written_bytes, 0);
}
