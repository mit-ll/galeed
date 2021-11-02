use fakify_proc_macro::fakify_proc;
use fakeptr::{FakePtr};
use std::fs::File;
use std::io::Write;

#[fakify_proc]
struct MyStruct {
    x: i32,
    y: bool,
}

// TODO - Auto-generate FakePtr<T> definition from regular definition
extern {
    pub fn acton_mystruct_unsafe(p: *const MyStruct) -> u64;
    pub fn acton_mystruct_safe(p: FakePtr<MyStruct>) -> u64;
    pub fn pin_thread();
}


fn main() {
    unsafe {
        pin_thread();
    }
    for num_actions in vec![100_000, 200_000, 300_000, 400_000, 500_000, 600_000, 700_000, 800_000, 900_000, 1_000_000] {
        let mut unsafe_cycle_collection: Vec<u64> = vec![];
        let mut safe_cycle_collection: Vec<u64> = vec![];
        
        let test_x = 7;
        let test_y = false;

        let my_struct_orig = MyStruct{x: test_x, y: test_y};
        for _ in 0..num_actions {
            let unsafe_cycles: u64;
            unsafe {
                unsafe_cycles = acton_mystruct_unsafe(&my_struct_orig);
            }
            unsafe_cycle_collection.push(unsafe_cycles);
        }


        let my_struct_safe = MyStruct{x: test_x, y: test_y};
        let my_fake_ptr = MyStruct::to_fake_ptr(my_struct_safe);
        for _ in 0..num_actions {
            let safe_cycles: u64;
            unsafe {
                safe_cycles = acton_mystruct_safe(my_fake_ptr);
            }
            safe_cycle_collection.push(safe_cycles);
        }
        let _my_struct_recovered = MyStruct::recover(my_fake_ptr);

        let mut file = File::create(format!("./benchmark_results/write_only/{}.csv", num_actions)).expect("Could not create file");
        for i in 0..num_actions {
            writeln!(file, "{},{}", unsafe_cycle_collection[i], safe_cycle_collection[i]).expect("Could not write to file");
        }

        let unsafe_avg = unsafe_cycle_collection.iter().sum::<u64>() as f64 / unsafe_cycle_collection.len() as f64;
        let safe_avg = safe_cycle_collection.iter().sum::<u64>() as f64 / safe_cycle_collection.len() as f64;
        println!("{:?},{:?},{:?},{:?}", num_actions, unsafe_avg, safe_avg, safe_avg / unsafe_avg);
    }
}