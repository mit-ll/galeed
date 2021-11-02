#![feature(asm)]

use std::alloc::{GlobalAlloc, Layout};
use core::arch::x86_64::{_rdtsc};
use std::fs::File;
use std::io::Write;

// ((0x0) << (2 * pkey)), where pkey is MPK_DOMAIN_PRIVATE_ID (but assumed to be 1 here);
const PKRU_ALLOW_READ_WRITE: u64 = 0x0;
const PKRU_ALLOW_WRITE: u64 = 0x4;
const PKRU_ALLOW_READ: u64 = 0x8;
const PKRU_DISABLE_ALL: u64 = 0xc;
static mut MPK_DOMAIN_PRIVATE_ID: i32 = -1;

extern {
    pub fn read_int_ptr(int_from_rust: *mut i32) -> u64;
    pub fn write_int_ptr(int_from_rust: *mut i32) -> u64;
    pub fn read_write_int_ptr(int_from_rust: *mut i32) -> u64;
    // pub fn __rdtscp() -> u64;

    pub fn mpk_create() -> i32;
    pub fn mpk_alloc(mpk_id: i32, size:u32) -> *mut u8;
    pub fn mpk_free(ptr: *mut u8);

    pub fn mpt_update(pkey: i32, prot: i32, synch: bool) -> i32;
}

struct MyAllocator;
unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if MPK_DOMAIN_PRIVATE_ID == -1 {
            MPK_DOMAIN_PRIVATE_ID = mpk_create();
        }
        mpk_alloc(MPK_DOMAIN_PRIVATE_ID, layout.size() as u32)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        mpk_free(ptr)
    }
}

#[global_allocator]
static GLOBAL: MyAllocator = MyAllocator;

#[repr(C)]
#[derive(Debug)]
pub struct Point {
    x: i32
}

fn main() {
    for num_actions in vec![100_000, 200_000, 300_000, 400_000, 500_000, 600_000, 700_000, 800_000, 900_000, 1_000_000] {
        let mut read_inner_cycle_collection: Vec<u64> = vec![];
        let mut read_outer_cycle_collection: Vec<u64> = vec![];
        let mut write_inner_cycle_collection: Vec<u64> = vec![];
        let mut write_outer_cycle_collection: Vec<u64> = vec![];
        let mut read_write_inner_cycle_collection: Vec<u64> = vec![];
        let mut read_write_outer_cycle_collection: Vec<u64> = vec![];

        for _ in 0..num_actions {
            let inner_cycles: u64;
            let outer_cycles: u64;
            let mut int_box = Box::new(7);
            let int_box_ptr: *mut i32 = &mut *int_box;
            unsafe {
                let start = _rdtsc();

                let mut eax: u64;
                let ecx: u64 = 0x0;
                let edx: u64 = 0x0;
                asm!("rdpkru", in("ecx") ecx, lateout("eax") eax, lateout("edx") _);
                eax = ((eax & !PKRU_DISABLE_ALL) | PKRU_ALLOW_READ);
                asm!("wrpkru", in("eax") eax, in("ecx") ecx, in("edx") edx);

                inner_cycles = read_int_ptr(int_box_ptr);

                asm!("rdpkru", in("ecx") ecx, lateout("eax") eax, lateout("edx") _);
                eax = ((eax & !PKRU_DISABLE_ALL) | PKRU_ALLOW_READ_WRITE);
                asm!("wrpkru", in("eax") eax, in("ecx") ecx, in("edx") edx);

                outer_cycles = _rdtsc() - start;
            }
            read_outer_cycle_collection.push(outer_cycles);
            read_inner_cycle_collection.push(inner_cycles);
        }

        for _ in 0..num_actions {
            let inner_cycles: u64;
            let outer_cycles: u64;
            let mut int_box = Box::new(7);
            let int_box_ptr: *mut i32 = &mut *int_box;
            unsafe {
                let start = _rdtsc();

                let mut eax: u64;
                let ecx: u64 = 0x0;
                let edx: u64 = 0x0;
                asm!("rdpkru", in("ecx") ecx, lateout("eax") eax, lateout("edx") _);
                eax = ((eax & !PKRU_DISABLE_ALL) | PKRU_ALLOW_READ_WRITE); //Of course it has to read to write, duh
                asm!("wrpkru", in("eax") eax, in("ecx") ecx, in("edx") edx);
                
                inner_cycles = write_int_ptr(int_box_ptr);

                asm!("rdpkru", in("ecx") ecx, lateout("eax") eax, lateout("edx") _);
                eax = ((eax & !PKRU_DISABLE_ALL) | PKRU_ALLOW_READ_WRITE);
                asm!("wrpkru", in("eax") eax, in("ecx") ecx, in("edx") edx);

                outer_cycles = _rdtsc() - start;
            }
            write_outer_cycle_collection.push(outer_cycles);
            write_inner_cycle_collection.push(inner_cycles);
        }

        for _ in 0..num_actions {
            let inner_cycles: u64;
            let outer_cycles: u64;
            let mut int_box = Box::new(7);
            let int_box_ptr: *mut i32 = &mut *int_box;
            unsafe {
                let start = _rdtsc();
                
                let mut eax: u64;
                let ecx: u64 = 0x0;
                let edx: u64 = 0x0;
                asm!("rdpkru", in("ecx") ecx, lateout("eax") eax, lateout("edx") _);
                eax = ((eax & !PKRU_DISABLE_ALL) | PKRU_ALLOW_READ_WRITE);
                asm!("wrpkru", in("eax") eax, in("ecx") ecx, in("edx") edx);

                inner_cycles = read_write_int_ptr(int_box_ptr);

                asm!("rdpkru", in("ecx") ecx, lateout("eax") eax, lateout("edx") _);
                eax = ((eax & !PKRU_DISABLE_ALL) | PKRU_ALLOW_READ_WRITE);
                asm!("wrpkru", in("eax") eax, in("ecx") ecx, in("edx") edx);

                outer_cycles = _rdtsc() - start;
            }
            read_write_outer_cycle_collection.push(outer_cycles);
            read_write_inner_cycle_collection.push(inner_cycles);
        }
        
        let mut read_file = File::create(format!("./benchmark_results/with_mpk/read/{}.csv", num_actions)).expect("Could not create file");
        for i in 0..num_actions {
            writeln!(read_file, "{},{}", read_inner_cycle_collection[i], read_outer_cycle_collection[i]).expect("Could not write to file");
        }
        let mut write_file = File::create(format!("./benchmark_results/with_mpk/write/{}.csv", num_actions)).expect("Could not create file");
        for i in 0..num_actions {
            writeln!(write_file, "{},{}", write_inner_cycle_collection[i], write_outer_cycle_collection[i]).expect("Could not write to file");
        }
        let mut read_write_file = File::create(format!("./benchmark_results/with_mpk/read_write/{}.csv", num_actions)).expect("Could not create file");
        for i in 0..num_actions {
            writeln!(read_write_file, "{},{}", read_write_inner_cycle_collection[i], read_write_outer_cycle_collection[i]).expect("Could not write to file");
        }
    
    // let mut point_box = Box::new(Point{x: 15});
    // let point_box_ptr: *mut Point = &mut *point_box;
    // println!("Point box value: {:?}", point_box);
    // println!("Point box ptr: {:?}", point_box_ptr);

    // unsafe {
    //     mpt_update(MPK_DOMAIN_PRIVATE_ID, 2, true); // 0 = no permission, 1 = read-only, 2 = read-write
    //     cpp_malloc_test2(point_box_ptr);
    //     mpt_update(MPK_DOMAIN_PRIVATE_ID, 2, true);
    // }

    // point_box.x = 5; // To prove that I can still modify things outside of C++
    // println!("Point box value: {:?}", point_box);
    // println!("Point box ptr: {:?}", point_box_ptr);
    }
}
