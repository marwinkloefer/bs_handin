/*****************************************************************************
 *                                                                           *
 *                  F R A M E S                                              *
 *                                                                           *
 *---------------------------------------------------------------------------*
 * Beschreibung:    Verwaltung der Page-Frames in zwei Listen:               *
 *                     - Kernel-Page-Frames: 0 .. 64 MiB - 1                 *
 *                     - User-Page-Frames:   >= 64 MiB                       *
 *                  Der Code ist eine angepasste Version des ListAllocators. *
 *                  Wir verwalten hier auch Speicherbloecke, deren Start-    *
 *                  Adresse aber immer 4 KB aliginiert sind und deren Groesse*
 *                  immer 4 KB oder ein Vielfaches davon sind. Zudem werden  *
 *                  die Metadaten direkt in dem freien Page-Frame gespeichert*
 *                  und die Liste ist aufsteigend sortiert nach den          *
 *                  Startadressen der Bloecke. Durch die Sortierung ist eine *
 *                  Verschmelzung bei der Freigabe einfach moeglich.         *
 *                                                                           *
 * Autor:           Michael Schoettner, 21.1.2024                            *
 *****************************************************************************/

use core::num;
use core::ops::Add;
use core::slice;
use core::{mem, ptr};

use alloc::alloc::Layout;
use alloc::string::ToString;
use alloc::vec::Vec;

use crate::boot::multiboot::PhysRegion;
use crate::consts::KERNEL_PHYS_SIZE;
use crate::consts::PAGE_FRAME_SIZE;
use crate::devices::kprint;
use crate::kernel::allocator::list::PfListAllocator;

// letzte nutzbare physikalische Adresse
// (notwendig fuer das 1:1 mapping des Kernels in den Page-Tables)
// TODO checken ob hier nicht dynamisch ausgelesen werden müsste in makefile.toml steht 256M
static mut MAX_PHYS_ADDR: PhysAddr = PhysAddr(0);

// Page-Frames > KERNEL_VM_SIZE
static mut FREE_USER_PAGE_FRAMES: PfListAllocator = PfListAllocator::new();

// Page-Frames 0 .. KERNEL_VM_SIZE - 1
static mut FREE_KERNEL_PAGE_FRAMES: PfListAllocator = PfListAllocator::new();

// Eine physikalische Adresse
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[repr(transparent)]
pub struct PhysAddr(pub u64);

impl PhysAddr {
    pub fn new(addr: u64) -> PhysAddr {
        Self(addr)
    }
    
    pub fn to_start_address(&self) -> usize {
        self.raw() as usize
    }

    pub fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    pub fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    pub fn raw(&self) -> u64 {
        self.0
    }

    pub fn get_max_phys_addr() -> PhysAddr {
        unsafe {
            MAX_PHYS_ADDR
        }
    }
}

impl core::fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Phys(0x{:x})", self.0)
    }
}

impl From<PhysAddr> for u64 {
    fn from(addr: PhysAddr) -> Self {
        addr.0
    }
}

impl Add<PhysAddr> for PhysAddr {
    type Output = PhysAddr;

    fn add(self, rhs: PhysAddr) -> Self::Output {
        let res = (self.0.checked_add(rhs.0).unwrap()) as u64;
        PhysAddr(res)
    }
}


// Initialisiert die Page-Frame-Liste anhand der uebergebenen freien Memory-Regionen
// Bei Bedarf werden die Memory-Regionen angepasst, sodass die Startadresse
// 4 KB aliginiert ist und auch die Grösse 4 KB oder ein Vielfaches davon ist
pub fn pf_init(free: &mut Vec<PhysRegion>) {
    unsafe {
        // MAX_PHYS_ADDR setzen
        for region in free.iter() {
            let end_addr = region.end;
            if end_addr > MAX_PHYS_ADDR.0 {
                MAX_PHYS_ADDR = PhysAddr(end_addr);
            }
        }
        kprintln!("pf_init: max phys addr = 0x{:x}", MAX_PHYS_ADDR.raw());
        FREE_USER_PAGE_FRAMES.init(free, false);   
        FREE_USER_PAGE_FRAMES.dump_free_list("user".to_string());  
        FREE_KERNEL_PAGE_FRAMES.init(free, true);     
        FREE_KERNEL_PAGE_FRAMES.dump_free_list("kernel".to_string());
    }
}

pub fn pf_dump_lists(){
    unsafe {
        FREE_USER_PAGE_FRAMES.dump_free_list("user".to_string());  
        FREE_KERNEL_PAGE_FRAMES.dump_free_list("kernel".to_string());
    }
}


// Alloziere 'pf_count' aufeinanderfolgende Page-Frames
// Vom Kernel-Space, falls 'in_kernel_space' = true
// Oder User-Space, falls 'in_kernel_space' = false
pub fn pf_alloc(pf_count: usize, in_kernel_space: bool) -> PhysAddr {
    unsafe {
        if in_kernel_space {
            // Kernel-Space alloc
            //kprintln!("allocate {} pages with each size 4kb (size: {} bytes) in kernel space", pf_count, pf_count*PAGE_FRAME_SIZE);
            PhysAddr::new(FREE_KERNEL_PAGE_FRAMES.alloc(pf_count) as u64)
        } else {
            // User-Space alloc
            //kprintln!("allocate {} pages with each size 4kb (size: {} bytes) in user space", pf_count, pf_count*PAGE_FRAME_SIZE);
            PhysAddr::new(FREE_USER_PAGE_FRAMES.alloc(pf_count) as u64)
        }
    }
}

// Gebe 'pf_count' aufeinanderfolgende Page-Frames frei
// Zuordnung User- oder Kernel-Space ergibt sich anhand der Adresse
pub fn pf_free(pf_addr: PhysAddr, pf_count: usize) {
    unsafe{
        let ptr_u8 = pf_addr.raw() as *mut u8;
        if pf_addr.raw() < 64 * 1024 * 1024 { // kernel dealloc
            kprintln!("deallocate {} pages with each size 4kb (space: {} bytes) in kernel space at address=0x{:x}", pf_count, pf_count*PAGE_FRAME_SIZE, pf_addr.raw());
            FREE_KERNEL_PAGE_FRAMES.dealloc(ptr_u8, pf_count);
        } else { // user dealloc
            kprintln!("deallocate {} pages with each size 4kb (space: {} bytes) in user space at address=0x{:x}", pf_count, pf_count*PAGE_FRAME_SIZE, pf_addr.raw());
            FREE_USER_PAGE_FRAMES.dealloc(ptr_u8, pf_count);
        }
    }
}
