use memflow::error::AsIntResult;
use memflow::mem::phys_mem::*;
use memflow::plugins::ConnectorInstance;
use memflow::types::PhysicalAddress;

use std::slice::{from_raw_parts, from_raw_parts_mut};

/// Read a list of values
///
/// This will perform `len` physical memory reads on the provided `data`. Using lists is preferable
/// for performance, because then the underlying connectors can batch those operations.
///
/// # Safety
///
/// `data` must be a valid array of `PhysicalReadData` with the length of at least `len`
#[no_mangle]
pub unsafe extern "C" fn phys_read_raw_list(
    mem: &mut ConnectorInstance,
    data: *mut PhysicalReadData,
    len: usize,
) -> i32 {
    let data = from_raw_parts_mut(data, len);
    mem.phys_read_raw_list(data).as_int_result()
}

/// Write a list of values
///
/// This will perform `len` physical memory writes on the provided `data`. Using lists is preferable
/// for performance, because then the underlying connectors can batch those operations.
///
/// # Safety
///
/// `data` must be a valid array of `PhysicalWriteData` with the length of at least `len`
#[no_mangle]
pub unsafe extern "C" fn phys_write_raw_list(
    mem: &mut ConnectorInstance,
    data: *const PhysicalWriteData,
    len: usize,
) -> i32 {
    let data = from_raw_parts(data, len);
    mem.phys_write_raw_list(data).as_int_result()
}

/// Retrieve metadata about the physical memory object
#[no_mangle]
pub extern "C" fn phys_metadata(mem: &ConnectorInstance) -> PhysicalMemoryMetadata {
    mem.metadata()
}

/// Read a single value into `out` from a provided `PhysicalAddress`
///
/// # Safety
///
/// `out` must be a valid pointer to a data buffer of at least `len` size.
#[no_mangle]
pub unsafe extern "C" fn phys_read_raw(
    mem: &mut ConnectorInstance,
    addr: PhysicalAddress,
    out: *mut u8,
    len: usize,
) -> i32 {
    mem.phys_read_raw_into(addr, from_raw_parts_mut(out, len))
        .as_int_result()
}

/// Read a single 32-bit value from a provided `PhysicalAddress`
#[no_mangle]
pub extern "C" fn phys_read_u32(mem: &mut ConnectorInstance, addr: PhysicalAddress) -> u32 {
    mem.phys_read::<u32>(addr).unwrap_or_default()
}

/// Read a single 64-bit value from a provided `PhysicalAddress`
#[no_mangle]
pub extern "C" fn phys_read_u64(mem: &mut ConnectorInstance, addr: PhysicalAddress) -> u64 {
    mem.phys_read::<u64>(addr).unwrap_or_default()
}

/// Write a single value from `input` into a provided `PhysicalAddress`
///
/// # Safety
///
/// `input` must be a valid pointer to a data buffer of at least `len` size.
#[no_mangle]
pub unsafe extern "C" fn phys_write_raw(
    mem: &mut ConnectorInstance,
    addr: PhysicalAddress,
    input: *const u8,
    len: usize,
) -> i32 {
    mem.phys_write_raw(addr, from_raw_parts(input, len))
        .as_int_result()
}

/// Write a single 32-bit value into a provided `PhysicalAddress`
#[no_mangle]
pub extern "C" fn phys_write_u32(
    mem: &mut ConnectorInstance,
    addr: PhysicalAddress,
    val: u32,
) -> i32 {
    mem.phys_write(addr, &val).as_int_result()
}

/// Write a single 64-bit value into a provided `PhysicalAddress`
#[no_mangle]
pub extern "C" fn phys_write_u64(
    mem: &mut ConnectorInstance,
    addr: PhysicalAddress,
    val: u64,
) -> i32 {
    mem.phys_write(addr, &val).as_int_result()
}
