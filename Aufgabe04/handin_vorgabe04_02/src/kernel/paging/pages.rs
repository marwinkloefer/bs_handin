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

// Bezügliche der Seitentabelleneinträge lassen wir vorerst alle Einträge im Ring 3 zugreifbar, löschen also nicht das User-Bit.
// Zudem setzen wir alle Seiten auf schreibbar und sofern mit Page-Frames unterlegt auf „Präsent“.
// Um andere mögliche Bits in den Seitentabelleneinträgen, wie Caching, No-Execute, Protection Keys etc., kümmern wir uns nicht.
impl PTEflags {
    fn flags_for_kernel_pages() -> Self {
        /*
        *   Bezügliche der Seitentabelleneinträge lassen wir vorerst alle Einträge im Ring 3 zugreifbar, löschen
        *   also nicht das User-Bit. Das ist noch notwendig, damit wir den Code im Ring 3 ausführen können,
        *   wird aber in einem späteren Übungsblatt abgeschafft.
        */
        PTEflags::PRESENT | PTEflags::WRITEABLE | PTEflags::GLOBAL | PTEflags::USER
    }

    fn flags_for_kernel_int_pages_user_present() -> Self {
        PTEflags::PRESENT | PTEflags::WRITEABLE | PTEflags::GLOBAL | PTEflags::USER
    }

    fn flags_for_kernel_page_zero() -> Self {
        PTEflags::WRITEABLE | PTEflags::GLOBAL | PTEflags::USER
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
        unsafe {
            x86::controlregs::cr3_write(addr.into());
        }
    }

    // Diese Funktion richtet ein neues Mapping ein
    // 'vm_addr':     virtuelle Startaddresse des Mappings
    // 'nr_of_pages': Anzahl der Seiten, die ab 'vm_addr' gemappt werden sollen
    fn mmap_kernel(&mut self, vm_addr: usize, nr_of_pages: usize) { // war mal "mut vm_addr: usize" aber compiler hat geraten dass unmute
    // bekommen eine 4kb seite der pml4
    // hier muss jetzt an der entsrechenden stelle, die adresse der pdpte eingetragen werden
        Self::create_pdpt_in_pml4_kernel(self, vm_addr, nr_of_pages);
    }

    // Diese Funktion richtet ein neues Mapping ein
    // 'vm_addr':     virtuelle Startaddresse des Mappings
    // 'nr_of_pages': Anzahl der Seiten, die ab 'vm_addr' gemappt werden sollen
    fn mmap_user(&mut self, vm_addr: usize, nr_of_pages: usize) { // war mal "mut vm_addr: usize" aber compiler hat geraten dass unmute
    // bekommen eine 4kb seite der pml4
    // hier muss jetzt an der entsrechenden stelle, die adresse der pdpte eingetragen werden
        Self::create_pdpt_in_pml4_user(self, vm_addr, nr_of_pages);
    }

// ####################################################################################################################################
// ##################################################### PML4 mit PDPT verdrahten #####################################################
// ####################################################################################################################################
    fn create_pdpt_in_pml4_kernel(pml4: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) {
        kprintln!("create_pdpt_in_pml4_kernel: level=4, {} Seiten ab 0x{:x} zu mappen.",nr_of_pages,start_vm_addr);
    
        // ---------------------------------------------------------
        // 1. index bestimmen PML4 => Bits 47..39 (also >> 39 & 0x1FF)
        // ---------------------------------------------------------
        let index_pml4 = ((start_vm_addr >> 39) & 0x1ff) as usize;
    
        // ---------------------------------------------------------
        // 2. neue tabelle allozieren und als PDPT initialisieren
        // ---------------------------------------------------------
        // Physikalische Seite für PDPT anfordern
        let pd_frame = frames::pf_alloc(1, true);
        assert!(pd_frame != PhysAddr(0),"pf_alloc() für PDPT schlug fehl oder lieferte 0!");
        // page table pointer auf adresse erstellen
        let pdpt_virt_ptr = pd_frame.as_mut_ptr::<PageTable>();

        // ---------------------------------------------------------
        // 3. tabelle nullen
        // ---------------------------------------------------------
        // Tabelle als 0 gefüllte tabelle initialisieren an erhaltener adresse
        let pdpt_table = unsafe { &mut *pdpt_virt_ptr };
        for entry in pdpt_table.entries.iter_mut() {
            *entry = PageTableEntry(0);
        }

        // ---------------------------------------------------------
        // 4. pdpt in pml4 eintragen
        // ---------------------------------------------------------
        // neune Eintrag in pml4 der auf neu erstelle pdpt tabelle refferenziert
        pml4.entries[index_pml4] = PageTableEntry::new(pd_frame, PTEflags::flags_for_kernel_pages());
    
        // ---------------------------------------------------------
        // 5. nächste Funktion delegieren
        // ---------------------------------------------------------
        let unmapped = Self::create_pd_in_pdpt_kernel(pdpt_table, start_vm_addr, nr_of_pages);
    
        // ---------------------------------------------------------
        // 6. falls überlauf, rekusiv erneut aufrufen
        // ---------------------------------------------------------
        if unmapped > 0 {
            if index_pml4 == 511 {
                // Kein weiterer eintrag in diesem PD verfügbar -> muss neuen pd anlegen
                assert!(index_pml4 != 511,"create_pdpt_in_pml4_kernel() limit für vierstufiges paging erreicht! Alle 512 Einträge in PML4 aufgebraucht!");
            } else {
                return Self::create_pdpt_in_pml4_kernel(pml4, start_vm_addr + (nr_of_pages - unmapped) * PAGE_SIZE, unmapped);
            }
        }
    }
    
// ####################################################################################################################################
// ###################################################### PDPT mit PD verdrahten ######################################################
// ####################################################################################################################################
    fn create_pd_in_pdpt_kernel(pdpt: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        kprintln!("create_pd_in_pdpt_kernel: level=3, {} viele seiten ab 0x{:x} zu mappen.", nr_of_pages, start_vm_addr);
        // ---------------------------------------------------------
        // 1. index bestimmen pdpt => Bits 38...30 (also >> 3o & 0x1FF)
        // ---------------------------------------------------------
        let index_pdpt = ((start_vm_addr >> 30) & 0x1ff) as usize;
    
        // ---------------------------------------------------------
        // 2. neue tabelle allozieren und als pd initialisieren
        // ---------------------------------------------------------
        // Physikalische Seite für pd anfordern
        let pd_frame = frames::pf_alloc(1, true);
        assert!(pd_frame != PhysAddr(0),"pf_alloc() für pd schlug fehl oder lieferte 0!");
        // page table pointer auf adresse erstellen
        let pd_virt_ptr = pd_frame.as_mut_ptr::<PageTable>();

        // ---------------------------------------------------------
        // 3. tabelle nullen
        // ---------------------------------------------------------
        // Tabelle als 0 gefüllte tabelle initialisieren an erhaltener adresse
        let pd_table = unsafe { &mut *pd_virt_ptr };
        for entry in pd_table.entries.iter_mut() {
            *entry = PageTableEntry(0);
        }

        // ---------------------------------------------------------
        // 4. pd in pdpt eintragen
        // ---------------------------------------------------------
        // neune Eintrag in pdpt der auf neu erstelle pd tabelle refferenziert
        pdpt.entries[index_pdpt] = PageTableEntry::new(pd_frame, PTEflags::flags_for_kernel_pages());
    
        // ---------------------------------------------------------
        // 5. nächste Funktion delegieren
        // ---------------------------------------------------------
        let unmapped = Self::create_pt_in_pd_kernel(pd_table, start_vm_addr, nr_of_pages);
    
        // ---------------------------------------------------------
        // 6. falls überlauf, rekusiv erneut aufrufen
        // ---------------------------------------------------------
        if unmapped > 0 {
            if index_pdpt == 511 {
                // Kein weiterer eintrag in diesem PD verfügbar -> muss neuen pd anlegen
                return unmapped;
            } else {
                return Self::create_pd_in_pdpt_kernel(pdpt, start_vm_addr + (nr_of_pages - unmapped) * PAGE_SIZE, unmapped);
            }
        }
        return 0
    }

// ####################################################################################################################################
// ####################################################### PD mit PT verdrahten #######################################################
// ####################################################################################################################################
    fn create_pt_in_pd_kernel(pd: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        kprintln!("create_pt_in_pd_kernel: level=2, {} viele seiten ab 0x{:x} zu mappen.", nr_of_pages, start_vm_addr);
        // ---------------------------------------------------------
        // 1. index bestimmen pd => Bits 29..21 (also >> 21 & 0x1FF)
        // ---------------------------------------------------------
        let index_pd = ((start_vm_addr >> 21) & 0x1ff) as usize;
    
        // ---------------------------------------------------------
        // 2. neue tabelle allozieren und als pt initialisieren
        // ---------------------------------------------------------
        // Physikalische Seite für pt anfordern
        let pt_frame = frames::pf_alloc(1, true);
        assert!(pt_frame != PhysAddr(0),"pf_alloc() für pt schlug fehl oder lieferte 0!");
        // page table pointer auf adresse erstellen
        let pt_virt_ptr = pt_frame.as_mut_ptr::<PageTable>();

        // ---------------------------------------------------------
        // 3. tabelle nullen
        // ---------------------------------------------------------
        // Tabelle als 0 gefüllte tabelle initialisieren an erhaltener adresse
        let pt_table = unsafe { &mut *pt_virt_ptr };
        for entry in pt_table.entries.iter_mut() {
            *entry = PageTableEntry(0);
        }

        // ---------------------------------------------------------
        // 4. pdpt in pml4 eintragen
        // ---------------------------------------------------------
        // neune Eintrag in pml4 der auf neu erstelle pdpt tabelle refferenziert
        pd.entries[index_pd] = PageTableEntry::new(pt_frame, PTEflags::flags_for_kernel_pages());
    
        // ---------------------------------------------------------
        // 5. nächste Funktion delegieren
        // ---------------------------------------------------------
        let unmapped = Self::map_pages_in_pt_kernel(pt_table, start_vm_addr, nr_of_pages);
    
        // ---------------------------------------------------------
        // 6. falls überlauf, rekusiv erneut aufrufen oder zurück geben
        // ---------------------------------------------------------
        if unmapped > 0 {
            if index_pd == 511 {
                // Kein weiterer eintrag in diesem PD verfügbar -> muss neuen pd anlegen
                return unmapped;
            } else {
                return Self::create_pt_in_pd_kernel(pd, start_vm_addr + (nr_of_pages - unmapped) * PAGE_SIZE, unmapped);
            }
        }
        return 0
    }

// ####################################################################################################################################
// ##################################################### PT mit Seiten verdrahten #####################################################
// ####################################################################################################################################
    fn map_pages_in_pt_kernel(pt: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        kprintln!("map_pages_in_pt_kernel: level=1, {} viele seiten ab 0x{:x} zu mappen.", nr_of_pages, start_vm_addr);

        // index in der PT (unterste Ebene) ermitteln Bits [20:12] der virtuellen Adresse → (vm_addr >> 12) & 0x1ff
        let pt_index = ((start_vm_addr >> 12) & 0x1ff) as usize;
    
        // wieviele einträge haben wir ab pt_index noch frei, bis die Tabellenende (512 Einträge) 
        let max_entries = PAGE_TABLE_ENTRIES - pt_index;
        let pages_to_map = core::cmp::min(nr_of_pages, max_entries);
        let flags_kernel_present_accessuser = PTEflags::flags_for_kernel_int_pages_user_present(); 

        let first_address = start_vm_addr + 0 * PAGE_SIZE;
        let last_address = start_vm_addr + pages_to_map * PAGE_SIZE;
        for i in 0..pages_to_map {
            let vm_addr  = start_vm_addr + i * PAGE_SIZE;
            let phys_addr = vm_addr;    // Identity: 1:1
    
            // In der PT eintragen
            pt.entries[pt_index + i].set_addr(PhysAddr::new(phys_addr as u64));
            if phys_addr == 0
            {
                pt.entries[pt_index + i].set_flags(PTEflags::flags_for_kernel_page_zero());
                kprintln!("###### map_pages_in_pt_kernel: special case address 0 auf nicht present");
            } else {
                pt.entries[pt_index + i].set_flags(flags_kernel_present_accessuser);
            }
        }
    
        // anzahl nicht-gemappter Seiten zurückgeben, falls nr_of_pages > pages_to_map nicht alle seiten gemappt
        kprintln!(
            "map_pages_in_pt_kernel: {} viele gemappte ab adress 0x{:x} bis Letzte 0x{:x}. (todo: {} pages)", 
                pages_to_map, 
                first_address, 
                last_address,
                nr_of_pages-pages_to_map
        );

        return nr_of_pages - pages_to_map;
    }	

    //############################################################################################################################################
    //################################################################### User ###################################################################
    //############################################################################################################################################

    // ####################################################################################################################################
    // ##################################################### PML4 mit PDPT verdrahten #####################################################
    // ####################################################################################################################################
    fn create_pdpt_in_pml4_user(pml4: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) {
        kprintln!("create_pdpt_in_pml4_user: level=4, {} Seiten ab 0x{:x} zu mappen.",nr_of_pages,start_vm_addr);

        // ---------------------------------------------------------
        // 1. index bestimmen PML4 => Bits 47..39 (also >> 39 & 0x1FF)
        // ---------------------------------------------------------
        let index_pml4 = ((start_vm_addr >> 39) & 0x1ff) as usize;

        // ---------------------------------------------------------
        // 2. neue tabelle allozieren und als PDPT initialisieren
        // ---------------------------------------------------------
        // Physikalische Seite für PDPT anfordern
        let pd_frame = frames::pf_alloc(1, true);
        assert!(pd_frame != PhysAddr(0),"pf_alloc() für PDPT schlug fehl oder lieferte 0!");
        // page table pointer auf adresse erstellen
        let pdpt_virt_ptr = pd_frame.as_mut_ptr::<PageTable>();

        // ---------------------------------------------------------
        // 3. tabelle nullen
        // ---------------------------------------------------------
        // Tabelle als 0 gefüllte tabelle initialisieren an erhaltener adresse
        let pdpt_table = unsafe { &mut *pdpt_virt_ptr };
        for entry in pdpt_table.entries.iter_mut() {
            *entry = PageTableEntry(0);
        }

        // ---------------------------------------------------------
        // 4. pdpt in pml4 eintragen
        // ---------------------------------------------------------
        // neune Eintrag in pml4 der auf neu erstelle pdpt tabelle refferenziert
        pml4.entries[index_pml4] = PageTableEntry::new(pd_frame, PTEflags::flags_for_user_pages());

        // ---------------------------------------------------------
        // 5. nächste Funktion delegieren
        // ---------------------------------------------------------
        let unmapped = Self::create_pd_in_pdpt_user(pdpt_table, start_vm_addr, nr_of_pages);

        // ---------------------------------------------------------
        // 6. falls überlauf, rekusiv erneut aufrufen
        // ---------------------------------------------------------
        if unmapped > 0 {
            if index_pml4 == 511 {
                // Kein weiterer eintrag in diesem PD verfügbar -> muss neuen pd anlegen
                assert!(index_pml4 != 511,"create_pdpt_in_pml4_user() limit für vierstufiges paging erreicht! Alle 512 Einträge in PML4 aufgebraucht!");
            } else {
                return Self::create_pdpt_in_pml4_user(pml4, start_vm_addr + (nr_of_pages - unmapped) * PAGE_SIZE, unmapped);
            }
        }
    }

    // ####################################################################################################################################
    // ###################################################### PDPT mit PD verdrahten ######################################################
    // ####################################################################################################################################
    fn create_pd_in_pdpt_user(pdpt: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        kprintln!("create_pd_in_pdpt_user: level=3, {} viele seiten ab 0x{:x} zu mappen.", nr_of_pages, start_vm_addr);
        // ---------------------------------------------------------
        // 1. index bestimmen pdpt => Bits 38...30 (also >> 3o & 0x1FF)
        // ---------------------------------------------------------
        let index_pdpt = ((start_vm_addr >> 30) & 0x1ff) as usize;

        // ---------------------------------------------------------
        // 2. neue tabelle allozieren und als pd initialisieren
        // ---------------------------------------------------------
        // Physikalische Seite für pd anfordern
        let pd_frame = frames::pf_alloc(1, true);
        assert!(pd_frame != PhysAddr(0),"pf_alloc() für pd schlug fehl oder lieferte 0!");
        // page table pointer auf adresse erstellen
        let pd_virt_ptr = pd_frame.as_mut_ptr::<PageTable>();

        // ---------------------------------------------------------
        // 3. tabelle nullen
        // ---------------------------------------------------------
        // Tabelle als 0 gefüllte tabelle initialisieren an erhaltener adresse
        let pd_table = unsafe { &mut *pd_virt_ptr };
        for entry in pd_table.entries.iter_mut() {
            *entry = PageTableEntry(0);
        }

        // ---------------------------------------------------------
        // 4. pd in pdpt eintragen
        // ---------------------------------------------------------
        // neune Eintrag in pdpt der auf neu erstelle pd tabelle refferenziert
        pdpt.entries[index_pdpt] = PageTableEntry::new(pd_frame, PTEflags::flags_for_user_pages());

        // ---------------------------------------------------------
        // 5. nächste Funktion delegieren
        // ---------------------------------------------------------
        let unmapped = Self::create_pt_in_pd_user(pd_table, start_vm_addr, nr_of_pages);

        // ---------------------------------------------------------
        // 6. falls überlauf, rekusiv erneut aufrufen
        // ---------------------------------------------------------
        if unmapped > 0 {
            if index_pdpt == 511 {
                // Kein weiterer eintrag in diesem PD verfügbar -> muss neuen pd anlegen
                return unmapped;
            } else {
                return Self::create_pd_in_pdpt_user(pdpt, start_vm_addr + (nr_of_pages - unmapped) * PAGE_SIZE, unmapped);
            }
        }
        return 0
    }

    // ####################################################################################################################################
    // ####################################################### PD mit PT verdrahten #######################################################
    // ####################################################################################################################################
    fn create_pt_in_pd_user(pd: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        kprintln!("create_pt_in_pd_user: level=2, {} viele seiten ab 0x{:x} zu mappen.", nr_of_pages, start_vm_addr);
        // ---------------------------------------------------------
        // 1. index bestimmen pd => Bits 29..21 (also >> 21 & 0x1FF)
        // ---------------------------------------------------------
        let index_pd = ((start_vm_addr >> 21) & 0x1ff) as usize;

        // ---------------------------------------------------------
        // 2. neue tabelle allozieren und als pt initialisieren
        // ---------------------------------------------------------
        // Physikalische Seite für pt anfordern
        let pt_frame = frames::pf_alloc(1, true);
        assert!(pt_frame != PhysAddr(0),"pf_alloc() für pt schlug fehl oder lieferte 0!");
        // page table pointer auf adresse erstellen
        let pt_virt_ptr = pt_frame.as_mut_ptr::<PageTable>();

        // ---------------------------------------------------------
        // 3. tabelle nullen
        // ---------------------------------------------------------
        // Tabelle als 0 gefüllte tabelle initialisieren an erhaltener adresse
        let pt_table = unsafe { &mut *pt_virt_ptr };
        for entry in pt_table.entries.iter_mut() {
            *entry = PageTableEntry(0);
        }

        // ---------------------------------------------------------
        // 4. pdpt in pml4 eintragen
        // ---------------------------------------------------------
        // neune Eintrag in pml4 der auf neu erstelle pdpt tabelle refferenziert
        pd.entries[index_pd] = PageTableEntry::new(pt_frame, PTEflags::flags_for_user_pages());

        // ---------------------------------------------------------
        // 5. nächste Funktion delegieren
        // ---------------------------------------------------------
        let unmapped = Self::map_pages_in_pt_user(pt_table, start_vm_addr, nr_of_pages);

        // ---------------------------------------------------------
        // 6. falls überlauf, rekusiv erneut aufrufen oder zurück geben
        // ---------------------------------------------------------
        if unmapped > 0 {
            if index_pd == 511 {
                // Kein weiterer eintrag in diesem PD verfügbar -> muss neuen pd anlegen
                return unmapped;
            } else {
                return Self::create_pt_in_pd_user(pd, start_vm_addr + (nr_of_pages - unmapped) * PAGE_SIZE, unmapped);
            }
        }
        return 0
    }

    // ####################################################################################################################################
    // ##################################################### PT mit Seiten verdrahten #####################################################
    // ####################################################################################################################################

    /*
    * Hier muss Code eingefuegt werden
    *  Soll liegen für jeden Prozess an:
    *  virtuellen Adressbereich liegen (64 TiB bis 64 TiB + 64 KiB) 
    * 
    * use crate::consts::USER_STACK_VM_START;
    * use crate::consts::USER_STACK_VM_END;
    */

    fn map_pages_in_pt_user(pt: &mut PageTable, start_vm_addr: usize, nr_of_pages: usize) -> usize {
        kprintln!("map_pages_in_pt_user: level=1, {} viele seiten ab 0x{:x} zu mappen.", nr_of_pages, start_vm_addr);

        // index in der PT (unterste Ebene) ermitteln Bits [20:12] der virtuellen Adresse → (vm_addr >> 12) & 0x1ff
        let pt_index = ((start_vm_addr >> 12) & 0x1ff) as usize;

        // wieviele einträge haben wir ab pt_index noch frei, bis die Tabellenende (512 Einträge) 
        let max_entries = PAGE_TABLE_ENTRIES - pt_index;
        let pages_to_map = core::cmp::min(nr_of_pages, max_entries);
        let user_flags = PTEflags::flags_for_user_pages(); 

        // physikalischen Page-Frame (4KiB) anfordern
        let phys_addr = frames::pf_alloc(pages_to_map, false);
        assert!(phys_addr != PhysAddr(0), "Fehler: Keine freien Frames mehr!");

        let first_address = start_vm_addr + 0 * PAGE_SIZE;
        let last_address = start_vm_addr + pages_to_map * PAGE_SIZE;
        for i in 0..pages_to_map {
            let idx = pt_index + i;

            // PT aktualisieren
            pt.entries[idx].set_addr(phys_addr);
            pt.entries[pt_index + i].set_flags(user_flags);

        }

        // anzahl nicht-gemappter Seiten zurückgeben, falls nr_of_pages > pages_to_map nicht alle seiten gemappt
        kprintln!(
            "map_pages_in_pt_user: {} viele gemappte ab adress 0x{:x} bis Letzte 0x{:x}. (todo: {} pages)", 
                pages_to_map, 
                first_address, 
                last_address,
                nr_of_pages-pages_to_map
        );

        return nr_of_pages - pages_to_map;
    }

}


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
    kprintln!("pg_init_kernel_tables: returning pml4_addr = 0x{:x}, init done", pml4_addr.raw());   
    return pml4_addr;
}

// Diese Funktion richtet ein Mapping fuer den User-Mode Stack ein
pub fn pg_mmap_user_stack(pml4_addr: PhysAddr) -> *mut u8 {

    /*
    *  Soll liegen für jeden Prozess an:
    *  virtuellen Adressbereich liegen (64 TiB bis 64 TiB + 64 KiB) 
    */

    assert!(pml4_addr != PhysAddr(0));
    kprintln!("pml4_addr = {:?}", pml4_addr);

    // Type-Cast der pml4-Tabllenadresse auf "PageTable"
    let pml4_table;
    unsafe { pml4_table = &mut *(pml4_addr.as_mut_ptr::<PageTable>()) }

    // anzahl der benötigten seiten berechnen und anfordern
    let nr_of_pages = (STACK_SIZE) / PAGE_SIZE;
    pml4_table.mmap_user(USER_STACK_VM_START, nr_of_pages);

    // start adress des stack zurück geben
    let user_stack_vm_start_ptr = USER_STACK_VM_START as *mut u8;
    user_stack_vm_start_ptr
}

// Setze das CR3 Register
pub fn pg_set_cr3(pml4_addr: PhysAddr) {
    PageTable::set_cr3(pml4_addr);
}

