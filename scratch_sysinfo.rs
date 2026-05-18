use sysinfo::Disks;

fn main() {
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        println!("{:?}: {:?}", disk.name(), disk.mount_point());
        println!("Total: {}", disk.total_space());
        println!("Available: {}", disk.available_space());
    }
}
