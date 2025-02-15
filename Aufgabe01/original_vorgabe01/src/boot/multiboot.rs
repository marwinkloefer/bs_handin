/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: multiboot                                                       ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Contains functions for reading multiboot information.           ║
   ║         The function 'get_free_memory' needs an initialized allocator.  ║
   ║                                                                         ║
   ║         Structs are from Paun Stefan:                                   ║
   ║            https://github.com/paunstefan/mercury_os                     ║
   ║                                                                         ║
   ║         More information about multiboot can be found here:             ║
   ║       https://www.gnu.org/software/grub/manual/multiboot/multiboot.html ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Michael Schoetter, HHU Duesseldorf, 13.11.2023                  ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/
use crate::devices::kprint; // used to import code needed by kprintln!
use alloc::vec::Vec;
use core::fmt;
use core::mem::size_of;

// Beschreibt eine Region im physikalischen Adressraum
pub struct PhysRegion {
    pub start: u64,
    pub end: u64,
}

impl fmt::Debug for PhysRegion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PhysRegion [0x{:x}, 0x{:x}]", self.start, self.end)
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct MultibootInfo {
    pub flags: u32,
    pub mem_lower: u32,
    pub mem_upper: u32,
    pub boot_device: u32,
    pub cmdline: u32,
    pub mods_count: u32,
    pub mods_addr: u32,
    pub table: ELF_Section_Header_Table,
    pub mmap_length: u32,
    pub mmap_addr: u32,
    pub drives_length: u32,
    pub drives_addr: u32,
    pub config_table: u32,
    pub boot_loader_name: u32,
    pub apm_table: u32,
    pub vbe_control_info: u32,
    pub vbe_mode_info: u32,
    pub vbe_mode: u16,
    pub vbe_interface_seg: u16,
    pub vbe_interface_off: u16,
    pub vbe_interface_len: u16,
    pub framebuffer: MultibootFramebuffer,
}

impl MultibootInfo {
    /// Read the Multiboot info using 'info_address'
    /// Safety:
    /// The address given must point to a valid Multiboot structure
    pub const unsafe fn read(info_address: u64) -> &'static Self {
        &*((info_address) as *const Self) as _
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct MultibootFramebuffer {
    pub addr: u64,
    pub pitch: u32,
    pub width: u32,
    pub height: u32,
    pub bpp: u8,
    pub typ: u8,
    pub red_field_positon: u8,
    pub red_mask_size: u8,
    pub green_field_positon: u8,
    pub green_mask_size: u8,
    pub blue_field_positon: u8,
    pub blue_mask_size: u8,
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ELF_Section_Header_Table {
    pub num: u32,
    pub size: u32,
    pub addr: u32,
    pub shndx: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct MmapEntry {
    pub size: u32,
    pub addr: u64,
    pub len: u64,
    pub typ: u32,
}

#[derive(Debug)]
#[repr(u32)]
pub enum MmapType {
    Available = 1,
    Reserved = 2,
    Other,
}

impl From<u32> for MmapType {
    fn from(value: u32) -> Self {
        match value {
            1 => MmapType::Available,
            2 => MmapType::Reserved,
            _ => MmapType::Other,
        }
    }
}

//
// Ermittel freie Speicherbereiche im physikalischen Adressraum
//
pub fn get_free_memory(
    mbi_ptr: u64,
    kernel_region: PhysRegion,
    heap_region: PhysRegion,
) -> Vec<PhysRegion> {
    let mut free: Vec<PhysRegion> = Vec::new();
    let mut reserved: Vec<PhysRegion> = Vec::new();

    let mb_info: &MultibootInfo = unsafe { MultibootInfo::read(mbi_ptr) };
    let flags = mb_info.flags;

    // Informationen, welche Speicherbereiche genutzt werden koennen oder belegt sind
    // sammeln und passend in den beiden Vec-Strukturen speichern
    // Wir speichern immer Tupel (Start-, End-Adresse)
    if flags & 0x40 != 0 {
        let mmap_length = mb_info.mmap_length;
        let mmap_addr = mb_info.mmap_addr;

        unsafe {
            for i in 0..(mb_info.mmap_length / size_of::<MmapEntry>() as u32) {
                let mmap_entry = &*((mb_info.mmap_addr as u64) as *const MmapEntry).add(i as usize);
                let region = PhysRegion {
                    start: mmap_entry.addr,
                    end: mmap_entry.addr + mmap_entry.len - 1,
                };

                if mmap_entry.typ == 1 {
                    free.push(region);
                } else {
                    reserved.push(region);
                }
            }
        }
    } else {
        panic!("Multiboot did not provide mmap informations!");
    }

    //   kprintln!("   free: {:?}", free);
    //   kprintln!("   reserved: {:?}", reserved);

    // 0 - 1 MB ignorieren wir (hier Sachen vom BIOS sowie ACPI)
    let region_below_1mib = PhysRegion {
        start: 0,
        end: 0x100000 - 1,
    };
    reserved.push(region_below_1mib);

    // ab 1 MB ist der Kernel
    reserved.push(kernel_region);

    // Und danach der temporäre Heap
    reserved.push(heap_region);

    // 15 - 16 MB ignorieren wir (ISA hole)
    let region_isa = PhysRegion {
        start: 0xF0_0000,
        end: 0x100_0000 - 1,
    };
    reserved.push(region_isa);

    // Pruefen, ob reservierte und freien Speicherregionen ueberlappen
    // Und bei Bedarf freie Regionen zuschneiden
    let mut reserved_iter = reserved.iter();
    loop {
        let reserved_region = reserved_iter.next();
        if reserved_region.is_none() == true {
            break;
        }
        free = check_for_overlapping(free, reserved_region.unwrap());
        kprintln!("   free: {:?}", free);
    }

    kprintln!("   final free: {:?}", free);

    free
}

//
// Hilfsfunktion von 'get_free_memory'
// Hier wird geprueft, ob ein reservierter Speicherbereich [start_r, end_r]
// mit einer freien Speicherregion ueberlappt. Falls ja wird der als frei
// markierte Bereich, als reserviert angepasst.
fn check_for_overlapping(free: Vec<PhysRegion>, reserved_region: &PhysRegion) -> Vec<PhysRegion> {
    let mut free_then: Vec<PhysRegion> = Vec::new();
    let mut free_iter = free.iter();

    loop {
        let free_region_opt = free_iter.next();
        if free_region_opt.is_none() == true {
            break;
        }

        let free_region = free_region_opt.unwrap();

        // Liegt die freie Region komplett im Inneren der reservierten Region?
        // Falls ja, wird die freie Region komplett entfernt
        if reserved_region.start <= free_region.start && reserved_region.end >= free_region.end {
            // Hier machen wir nichts
        }
        // Liegt die reservierte Region komplett im Inneren der freien Region?
        // Falls ja, wird die freie Region in zwei freie Regionen aufgeteilt
        else if reserved_region.start > free_region.start && reserved_region.end < free_region.end
        {
            let free_region1 = PhysRegion {
                start: free_region.start,
                end: reserved_region.start - 1,
            };
            let free_region2 = PhysRegion {
                start: reserved_region.end + 1,
                end: free_region.end,
            };
            free_then.push(free_region1);
            free_then.push(free_region2);
        }
        // Ueberlappt die reservierte Region teilweise, ab dem Anfang der freien Region?
        else if reserved_region.end >= free_region.start && reserved_region.end < free_region.end
        {
            let free_region1 = PhysRegion {
                start: reserved_region.end + 1,
                end: free_region.end,
            };
            free_then.push(free_region1);
        }
        // Ueberlappt die reservierte Region teilweise, ab dem Ende der freien Region?
        else if reserved_region.start <= free_region.end
            && reserved_region.end > free_region.start
        {
            let free_region1 = PhysRegion {
                start: free_region.start,
                end: reserved_region.start - 1,
            };
            free_then.push(free_region1);
        }
        // Keine Ueberlappung (sollte der normale Fall sein)
        else {
            let free_region1 = PhysRegion {
                start: free_region.start,
                end: free_region.end,
            };
            free_then.push(free_region1);
        }
    }
    free_then
}

//
// Debug-Funktion zur Ausgabe verschiedener Multiboot-Infos
//
pub fn dump(mbi_ptr: u64) {
    let mb_info = unsafe { MultibootInfo::read(mbi_ptr) };
    let flags = mb_info.flags;

    kprintln!("Multiboot-Infos = {:x}", flags);
    kprintln!("   flags = {:x}", flags);

    // Allgemeine Speicherinfos
    if flags & 0x1 != 0 {
        let mem_lower = mb_info.mem_lower;
        let mem_upper = mb_info.mem_upper;
        kprintln!("   mem_lower = {} kB (memory below 1 MB)", mem_lower);
        kprintln!("   mem_upper = {} kB (memory above 1 MB)", mem_upper);
    }

    // Genaue Informationen, welche Speicherbereiche genutzt werden koennen oder belegt sind
    if flags & 0x40 != 0 {
        let mmap_length = mb_info.mmap_length;
        let mmap_addr = mb_info.mmap_addr;

        kprintln!("   mmap_addr = 0x{:x}", mmap_addr);
        kprintln!("   mmap_length = {} bytes ", mmap_length);
        kprintln!(
            "   mmap_entries = {} ",
            mmap_length / size_of::<MmapEntry>() as u32
        );
        unsafe {
            for i in 0..(mb_info.mmap_length / size_of::<MmapEntry>() as u32) {
                let mmap_entry = &*((mb_info.mmap_addr as u64) as *const MmapEntry).add(i as usize);

                kprintln!("      Entry {}: {:?}", i, mmap_entry);
            }
        }
        kprintln!("      mmap types:");
        kprintln!("              1 = available RAM");
        kprintln!("              3 = usable, holding ACPI infos");
        kprintln!("              4 = reserved");
        kprintln!("              5 = defect memory");
        kprintln!("              other numbers indicate reserved, unusable memory");
        kprintln!("");
    }
    // Framebuffer-Infos
    if flags & 0x1000 != 0 {
        let mb_fb: MultibootFramebuffer = mb_info.framebuffer;
        kprintln!("   framebuffer {:?}", mb_fb);
        /* vga::init( mb_info.framebuffer.addr,
          mb_info.framebuffer.pitch,
          mb_info.framebuffer.width,
          mb_info.framebuffer.height,
          mb_info.framebuffer.bpp
        ); */
    }
}
