/*****************************************************************************
 *                                                                           *
 *                  P A G E S                                                *
 *                                                                           *
 *---------------------------------------------------------------------------*
 * Beschreibung:    Hier sind die Funktionnen fuer die Page-Tables.          *
 *                                                                           *
 * Autor:           Michael Schoettner, 19.11.2024                           *
 *****************************************************************************/

use alloc::borrow::ToOwned;
use bitflags::bitflags;
use core::fmt;
use core::ops::BitOr;
use core::ptr;
use x86;

use crate::consts::PAGE_SIZE;
use crate::consts::STACK_SIZE;
use crate::consts::KERNEL_VM_SIZE;
use crate::consts::USER_STACK_VM_START;
use crate::consts::USER_STACK_VM_END;
use crate::kernel::paging::frames;
use crate::kernel::paging::frames::PhysAddr;


// Anzahl Eintraege in einer Seitentabelle
const PAGE_TABLE_ENTRIES: usize = 512;

// Flags eines Eintrages in der Seitentabelle
bitflags::bitflags! {
    pub struct PTEflags: u64 {
        const PRESENT = 1 << 0;
        const WRITEABLE = 1 << 1;
        const USER = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const CACHE_DISABLE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const HUGE_PAGE = 1 << 7;
        const GLOBAL = 1 << 8;
        const FREE = 1 << 9;          // Page-Entry free = 1, used = 0
    }
}

impl PTEflags {
    fn flags_for_kernel_pages() -> Self {

       /*
        * Hier muss Code eingefuegt werden
        *
        */

    }

    fn flags_for_user_pages() -> Self {

       /*
        * Hier muss Code eingefuegt werden
        *
        */

    }
}

// Page-Table-Eintrag
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    // Neuen Page-Table-Eintrag anlegen
    pub fn new(addr: PhysAddr, flags: PTEflags) -> Self {
        Self::new_internal(addr, flags | PTEflags::PRESENT)
    }

    fn new_internal(addr: PhysAddr, flags: PTEflags) -> Self {
        let addr: u64 = addr.into();
        Self(addr | flags.bits())
    }

    // Flags lesen
    pub fn get_flags(&self) -> PTEflags {
        PTEflags::from_bits_truncate(self.0)
    }

    // Flags schreiben
    pub fn set_flags(&mut self, flags: PTEflags) {
        *self = PageTableEntry::new_internal(self.get_addr(), flags);
        self.update();
    }

    // Adresse lesen
    pub fn get_addr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & 0x000f_ffff_ffff_f000)
    }

    // Setze die Adresse im Page-Table-Eintrag
    pub fn set_addr(&mut self, addr: PhysAddr) {
        *self = PageTableEntry::new_internal(addr, self.get_flags());
        self.update();
    }

    // Seite present?
    pub fn is_present(&self) -> bool {
        self.get_flags().contains(PTEflags::PRESENT)
    }

    // Free-Bit lesen
    pub(super) fn get_free(&self) -> bool {
        self.get_flags().contains(PTEflags::FREE)
    }

    // Free-Bit schreiben
    pub(super) fn set_free(&mut self, value: bool) {
        let mut flags = self.get_flags();
        if value {
            flags.insert(PTEflags::FREE);
        } else {
            flags.remove(PTEflags::FREE);
        }
        self.set_flags(flags);
        self.update();
    }

    // Änderungen in den Speicher durchschreiben
    fn update(&mut self) {
        let pe: *mut PageTableEntry = self;
        unsafe {
            pe.write(*pe);
        }
    }
}

impl core::fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "[addr={:?}, flags={:?}]",
            self.get_addr(),
            self.get_flags()
        )
    }
}


// Funktionen fuer die Page-Tables
#[repr(transparent)]
pub struct PageTable {
    pub entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {

    // Aktuelle Root-Tabelle auslesen
    pub fn get_cr3() -> PhysAddr {
        let cr3 = unsafe { x86::controlregs::cr3() };
        PhysAddr::new(cr3)
    }

    // Setze Root-Tabelle
    pub fn set_cr3(addr: PhysAddr) {
        unsafe {
            x86::controlregs::cr3_write(addr.into());
        }
    }

    // Diese Funktion richtet ein neues Mapping ein
    // 'vm_addr':     virtuelle Startaddresse des Mappings
    // 'nr_of_pages': Anzahl der Seiten, die ab 'vm_addr' gemappt werden sollen
    fn mmap_kernel(&mut self, mut vm_addr: usize, nr_of_pages: usize) {

       /*
        * Hier muss Code eingefuegt werden
        *
        */
     
		}
		
}


// Hier richten wir Paging-Tabellen ein, um den Kernel von 0 - KERNEL_SPACE 1:1 zu mappen
// Fuer die Page-Tables werden bei Bedarf Page-Frames alloziert
// CR3 wird am Ende gesetzt
pub fn pg_init_kernel_tables() -> PhysAddr {
    kprintln!("pg_init_kernel_tables");

    // Ausrechnen wie viel Seiten "gemappt" werden muessen
    let max_phys_addr: usize = PhysAddr::get_max_phys_addr().raw() as usize;
    let nr_of_pages = (max_phys_addr + 1) / PAGE_SIZE;
    kprintln!("   nr_of_pages = {}", nr_of_pages);
    kprintln!("   max_phys_addr = 0x{:x}", max_phys_addr);

    // Alloziere eine Tabelle fuer Page Map Level 4 (PML4) -> 4 KB
    let pml4_addr = frames::pf_alloc(1, true);
    assert!(pml4_addr != PhysAddr(0));
    //  kprintln!("pml4_addr = {:?}", pml4_addr);

    // Type-Cast der pml4-Tabllenadresse auf "PageTable"
    let pml4_table;
    unsafe { pml4_table = &mut *(pml4_addr.as_mut_ptr::<PageTable>()) }

    // Aufruf von "mmap"
    pml4_table.mmap(0, nr_of_pages);
       
    pml4_addr
}





// Diese Funktion richtet ein Mapping fuer den User-Mode Stack ein
pub fn pg_mmap_user_stack(pml4_addr: PhysAddr) -> *mut u8 {

    /*
     * Hier muss Code eingefuegt werden
     *
     */
     
}

// Setze das CR3 Register
pub fn pg_set_cr3(pml4_addr: PhysAddr) {
    PageTable::set_cr3(pml4_addr);
}

