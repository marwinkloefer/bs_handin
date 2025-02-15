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

use crate::consts::KERNEL_PHYS_SIZE;
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

// TODO
// Bezügliche der Seitentabelleneinträge lassen wir vorerst alle Einträge im Ring 3 zugreifbar, löschen also nicht das User-Bit.
// Zudem setzen wir alle Seiten auf schreibbar und sofern mit Page-Frames unterlegt auf „Präsent“.
// Um andere mögliche Bits in den Seitentabelleneinträgen, wie Caching, No-Execute, Protection Keys etc., kümmern wir uns nicht.
impl PTEflags {
    fn flags_for_kernel_pages() -> Self {
        PTEflags::PRESENT | PTEflags::WRITEABLE | PTEflags::GLOBAL
    }

    fn flags_for_user_pages() -> Self {
        PTEflags::PRESENT | PTEflags::WRITEABLE | PTEflags::GLOBAL | PTEflags::USER
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
        kprintln!("set_cr3: setze CR3 auf 0x{:x}", addr.raw());
        unsafe {
            x86::controlregs::cr3_write(addr.into());
        }
    }

    // Diese Funktion richtet ein neues Mapping ein
    // 'vm_addr':     virtuelle Startaddresse des Mappings
    // 'nr_of_pages': Anzahl der Seiten, die ab 'vm_addr' gemappt werden sollen
    fn mmap_kernel(&mut self, vm_addr: usize, nr_of_pages: usize) { // war mal "mut vm_addr: usize" aber compiler hat geraten dass unmute

        // vierstufiges Paging PML4 -> PDPT -> PD -> PT
        // Dabei gilt folgendes Bit-Schema für eine 48-Bit-Adressierung:
        //
        //   Bits:   47...........39  38...........30  29...........21  20...........12  11..........0
        //   Ebene:       PML4             PDPT             PD               PT              Offset
        //
        // in PML4E (Page map level 4)       Verweis auf PDPTE
        // in PDPTE (Page directory pointer) Verweis auf PDE
        // in PDE   (Page directory)         Verweis auf PTE
        // in PTE   (Page table)             Verweis auf physikalische Adresse (4kb)
        //
        // - Jede Ebene (PML4, PDPT, PD, PT) nutzt 9 Bit als Index 
        // - Der Offset innerhalb der 4KB-Page besteht aus den unteren 12 Bit (0..11)
        // - Somit ergeben sich 512 Einträge (2^9) pro Tabelle, 
        //   wobei jede Tabelle 4KB groß ist (512 Einträge * 8 Byte pro Eintrag = 4096 Byte)
        // - Das Register CR3 enthält die Basisadresse der PML4 (Page Map Level 4) für den aktuellen Kontext
        //
        // - virtuelle Seite = page / pages  (P)
        // - physikalishe Seite = page frame / page frames  (PF)
        // ===> 4 KB Pages und damit 4 KB Page-Frames ( 1 page frame speichert 1 page => 1:1 )

        let unmapped = Self::map_pages_rec(self, 4, vm_addr, nr_of_pages);

        if unmapped > 0 {
            kprintln!("Warnung: mmap_kernel: {} Seiten konnten nicht gemappt werden.", unmapped);
        } else {
            kprintln!("mmap_kernel: Erfolgreich {} Seiten ab 0x{:x} gemappt.", nr_of_pages, vm_addr);
        }
    }

    // mappe für jede ebene recursiv und gebe anzahl an noch zu mappender pages zurück
    fn map_pages_rec(current_table: &mut PageTable, level: u8, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        kprintln!("recursion: level={}, 0x{:x} viele seiten ab 0x{:x} zu mappen.", level, nr_of_pages, start_vm_addr);
        // basecase
        if nr_of_pages == 0 {return 0;}
        // allozieren von phys case
        if level == 1 {return Self::map_pages_pt(current_table, start_vm_addr, nr_of_pages);}


        // index der akatuellen Ebene bestimmen basierend auf start_vm_adresse
        let shift = 12 + 9 * (level - 1); // z.B. Level4 => shift=39
        let index = ((start_vm_addr >> shift) & 0x1ff) as usize;

        // allokier die nächste Tabelle
        let next_table_phys = frames::pf_alloc(1, true);
        assert!(next_table_phys != PhysAddr(0));

        // null-initialisieren
        let next_table = unsafe { &mut *(next_table_phys.as_mut_ptr::<PageTable>()) };
        for entry in next_table.entries.iter_mut() {
            *entry = PageTableEntry(0);
        }

        // eintrag setzen
        current_table.entries[index] = PageTableEntry::new(next_table_phys, PTEflags::flags_for_kernel_pages());

        // abstieg in die nächste Ebene ==>> nr_of_pages-1 , da wir ja eine 4kb seite für die aktuelle tabelle brauchen
        let unmapped_sub = Self::map_pages_rec(next_table, level - 1, start_vm_addr, nr_of_pages-1);

        // noch Seiten übrig -> (index + 1) bis am Ende der Tabelle (index=511) oder alle Seiten versorgt
        if unmapped_sub > 0 && index < 511 {
            // errechne virt add des nächsten eintrags: maskiere untere Bits von start_vm_addr und addieren (next_index << shift)
            let next_vm_addr = (start_vm_addr & !((1 << shift) - 1)) + (((index + 1) as usize) << shift);
            kprintln!("map_pages_rec: start_vm_addr = 0x{:x}, next_vm_addr = 0x{:x}", start_vm_addr, next_vm_addr);

            // rekursiver Aufruf auf Level um die restlichen unmapped_sub Seiten zu mappen.
            return Self::map_pages_rec(current_table, level, next_vm_addr, unmapped_sub);
        }
        return unmapped_sub;
    }


    fn map_pages_pml4(pt: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        return 0;
    }
    fn map_pages_pdpt(pt: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        return 0;
    }
    fn map_pages_pd(pt: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        return 0;
    }

    fn map_pages_pt(pt: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        // index in der PT (unterste Ebene) ermitteln Bits [20:12] der virtuellen Adresse → (vm_addr >> 12) & 0x1ff
        let pte_index = ((start_vm_addr >> 12) & 0x1ff) as usize;
    
        // wieviele einträge haben wir ab pte_index noch frei, bis die Tabellenende (512 Einträge) 
        let max_entries = PAGE_TABLE_ENTRIES - pte_index;
        let pages_to_map = core::cmp::min(nr_of_pages, max_entries);
        let kernel_flags = PTEflags::flags_for_kernel_pages(); 

        // physikalischen Page-Frame (4KiB) anfordern
        let frame_addr = frames::pf_alloc(pages_to_map, true);
        //assert!(frame_addr != PhysAddr(0), "Fehler: Keine freien Frames mehr!");

        // für jede zu mappende Seite → physischen Frame alloziieren + PT-Eintrag setzen
        for i in 0..pages_to_map {
            let idx = pte_index + i;
    
            // PT aktualisieren
            pt.entries[idx].set_addr(frame_addr);
            pt.entries[idx].set_flags(kernel_flags | PTEflags::PRESENT);
        }
    
        // anzahl nicht-gemappter Seiten zurückgeben, falls nr_of_pages > pages_to_map nicht alle seiten gemappt
        return nr_of_pages - pages_to_map;
    }


        // es empfiehlt sich eine rekursive Lösung
        // Die höchste physische Adresse ist nach dem Initialisieren des Page-Frame-Allokators bekannt. 
        // Für die Seitentabellen müssen Page-Frames alloziert werden, aber auf der untersten Eben nicht, da hier der bestehende physikalische Speicher nur „gemappt“ wird.
        // Bezügliche der Seitentabelleneinträge lassen wir vorerst alle Einträge im Ring 3 zugreifbar, löschen also nicht das User-Bit.
        // Zudem setzen wir alle Seiten auf schreibbar und sofern mit Page-Frames unterlegt auf „Präsent“.
        // Um andere mögliche Bits in den Seitentabelleneinträgen, wie Caching, No-Execute, Protection Keys etc., kümmern wir uns nicht.
        // Die erste Seite 0 sollte auf nicht-Präsent gesetzt werden, um Null-Pointer-Zugriffe abfangen und erkennen zu können.
        // Die Seitentabellen sollten zuerst ohne User-Mode Threads und ohne Interrupts getestet werden.
     
			
}


// Hier richten wir Paging-Tabellen ein, um den Kernel von 0 - KERNEL_SPACE 1:1 zu mappen
// Fuer die Page-Tables werden bei Bedarf Page-Frames alloziert
pub fn pg_init_kernel_tables() -> PhysAddr {
    kprintln!("pg_init_kernel_tables");

    // Ausrechnen wie viel Seiten "gemappt" werden muessen
    let max_phys_addr: usize = PhysAddr::get_max_phys_addr().raw() as usize;
    let nr_of_pages = (max_phys_addr) / PAGE_SIZE;
    // let nr_of_pages = (max_phys_addr + 1) / PAGE_SIZE;
    kprintln!("   nr_of_pages = {}", nr_of_pages);
    kprintln!("   max_phys_addr = 0x{:x}", max_phys_addr);

    // Alloziere eine Tabelle fuer Page Map Level 4 (PML4) -> 4 KB
    let pml4_addr = frames::pf_alloc(1, true);
    assert!(pml4_addr != PhysAddr(0));
    kprintln!("pml4_addr = {:?}", pml4_addr);

    // Type-Cast der pml4-Tabllenadresse auf "PageTable"
    let pml4_table;
    unsafe { pml4_table = &mut *(pml4_addr.as_mut_ptr::<PageTable>()) }

    pml4_table.mmap_kernel(0, nr_of_pages);
    kprintln!("pg_init_kernel_tables: returning pml4_table, init done");   
    return pml4_addr;
}



// TODO

// Diese Funktion richtet ein Mapping fuer den User-Mode Stack ein
pub fn pg_mmap_user_stack(pml4_addr: PhysAddr) -> *mut u8 {

    /*
     * Hier muss Code eingefuegt werden
     *
     */

    //dummy code zum compilieren
    let addr_as_ptr = (pml4_addr.0 as usize) as *mut u8;
    addr_as_ptr
}

// Setze das CR3 Register
pub fn pg_set_cr3(pml4_addr: PhysAddr) {
    kprintln!("pg_set_cr3: setze CR3 auf 0x{:x}", pml4_addr.raw());
    PageTable::set_cr3(pml4_addr);
}

