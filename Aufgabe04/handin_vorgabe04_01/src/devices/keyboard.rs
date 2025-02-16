/* ╔═════════════════════════════════════════════════════════════════════════╗
   ║ Module: keyboard                                                        ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Descr.: Here are the public functions of all modules implemented in the ║
   ║         keyboard sub directory.                                         ║
   ╟─────────────────────────────────────────────────────────────────────────╢
   ║ Author: Michael Schoetter, Univ. Duesseldorf, 6.5.2024                  ║
   ╚═════════════════════════════════════════════════════════════════════════╝
*/

use alloc::boxed::Box;
use core::sync::atomic::{AtomicU8, Ordering};
use spin::Mutex;

use crate::devices::cga;
use crate::devices::key;
use crate::kernel::cpu;
use crate::kernel::interrupts::int_dispatcher;
use crate::kernel::interrupts::isr;
use crate::kernel::interrupts::pic;

// called from mylib/input.rs
pub fn get_lastkey() -> u8 {
    LAST_KEY.swap(0, Ordering::SeqCst) as u8
}

// accessed by ISR, storing last read ASCII code
// and by get_lastkey, see above
static LAST_KEY: AtomicU8 = AtomicU8::new(0);

// Global thread-safe access to keyboard
static KB: Mutex<Keyboard> = Mutex::new(Keyboard {
    code: 0,
    prefix: 0,
    gather: key::Key {
        asc: 0,
        scan: 0,
        modi: 0,
    },
    leds: 0,
});

// Defining Keyboard struct
pub struct Keyboard {
    code: u8,         // Byte von Tastatur
    prefix: u8,       // Prefix von Tastatur
    gather: key::Key, // letzter dekodierter Key
    leds: u8,         // Zustand LEDs
}

/* Tabellen fuer ASCII-Codes intiialisieren */
static NORMAL_TAB: [u8; 89] = [
    0, 0, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48, 225, 39, 8, 0, 113, 119, 101, 114, 116, 122, 117,
    105, 111, 112, 129, 43, 13, 0, 97, 115, 100, 102, 103, 104, 106, 107, 108, 148, 132, 94, 0, 35,
    121, 120, 99, 118, 98, 110, 109, 44, 46, 45, 0, 42, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 45, 0, 0, 0, 43, 0, 0, 0, 0, 0, 0, 0, 60, 0, 0,
];

static SHIFT_TAB: [u8; 89] = [
    0, 0, 33, 34, 21, 36, 37, 38, 47, 40, 41, 61, 63, 96, 0, 0, 81, 87, 69, 82, 84, 90, 85, 73, 79,
    80, 154, 42, 0, 0, 65, 83, 68, 70, 71, 72, 74, 75, 76, 153, 142, 248, 0, 39, 89, 88, 67, 86,
    66, 78, 77, 59, 58, 95, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 62, 0, 0,
];

static ALT_TAB: [u8; 89] = [
    0, 0, 0, 253, 0, 0, 0, 0, 123, 91, 93, 125, 92, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 126,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 230, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 124, 0, 0,
];

static ASC_NUM_TAB: [u8; 13] = [55, 56, 57, 45, 52, 53, 54, 43, 49, 50, 51, 48, 44];

static SCAN_NUM_TAB: [u8; 13] = [8, 9, 10, 53, 5, 6, 7, 27, 2, 3, 4, 11, 51];

// Namen der LEDs
const LED_CAPS_LOCK: u8 = 4;
const LED_NUM_LOCK: u8 = 2;
const LED_SCROLL_LOCK: u8 = 1;

// Konstanten fuer die Tastaturdekodierung
const BREAK_BIT: u8 = 0x80;
const PREFIX1: u8 = 0xe0;
const PREFIX2: u8 = 0xe1;

// Benutzte Ports des Tastaturcontrollers
const KBD_CTRL_PORT: u16 = 0x64; // Status- (R) u. Steuerregister (W)
const KBD_DATA_PORT: u16 = 0x60; // Ausgabe- (R) u. Eingabepuffer (W)

// Bits im Statusregister des Tastaturcontrollers
const KBD_OUTB: u8 = 0x01;
const KBD_INPB: u8 = 0x02;
const KBD_AUXB: u8 = 0x20;

// Kommandos an die Tastatur
const KBD_CMD_SET_LED: u8 = 0xed;
const KBD_CMD_SET_SPEED: u8 = 0xf3;
const KBD_CMD_CPU_RESET: u8 = 0xfe;

// Antworten der Tastatur
const KBD_REPLY_ACK: u8 = 0xfa;

impl Keyboard {
    /*****************************************************************************
     * Funktion:        key_decoded                                              *
     *---------------------------------------------------------------------------*
     * Beschreibung:    Interpretiert die Make- und Break-Codes der Tastatur.    *
     *                                                                           *
     * Rueckgabewert:   true bedeutet, dass das Zeichen komplett ist             *
     *                  false es fehlen noch Make- oder Break-Codes.             *
     *****************************************************************************/
    fn key_decoded(&mut self) -> bool {
        let mut done: bool = false;

        // Die Tasten, die bei der MF II Tastatur gegenueber der aelteren
        // AT Tastatur hinzugekommen sind, senden immer erst eines von zwei
        // moeglichen Prefix Bytes.
        if self.code == PREFIX1 || self.code == PREFIX2 {
            self.prefix = self.code;
            return false;
        }

        // Das Loslassen einer Taste ist eigentlich nur bei den "Modifier" Tasten
        // SHIFT, CTRL und ALT von Interesse, bei den anderen kann der Break-Code
        // ignoriert werden.
        if (self.code & BREAK_BIT) != 0 {
            self.code &= !BREAK_BIT; // Der Break-Code einer Taste ist gleich dem
                                     // Make-Code mit gesetzten break_bit.
            match self.code {
                42 | 54 => {
                    self.gather.set_shift(false);
                }
                56 => {
                    if self.prefix == PREFIX1 {
                        self.gather.set_alt_right(false);
                    } else {
                        self.gather.set_alt_left(false);
                    }
                }
                29 => {
                    if self.prefix == PREFIX1 {
                        self.gather.set_ctrl_right(false);
                    } else {
                        self.gather.set_ctrl_left(false);
                    }
                }
                _ => { // alle anderen Tasten
                }
            }

            // Ein Prefix gilt immer nur fuer den unmittelbar nachfolgenden Code.
            // Also ist es jetzt abgehandelt.
            self.prefix = 0;

            // Mit einem Break-Code kann man nichts anfangen, also false liefern.
            return false;
        }

        // Eine Taste wurde gedrueckt. Bei den Modifier Tasten wie SHIFT, ALT,
        // NUM_LOCK etc. wird nur der interne Zustand geaendert. Durch den
        // Rueckgabewert 'false' wird angezeigt, dass die Tastatureingabe noch
        // nicht abgeschlossen ist. Bei den anderen Tasten werden ASCII
        // und Scancode eingetragen und ein 'true' fuer eine erfolgreiche
        // Tastaturabfrage zurueckgegeben, obwohl genaugenommen noch der Break-
        // code der Taste fehlt.

        match self.code {
            42 | 54 => {
                self.gather.set_shift(true);
            }
            56 => {
                if self.prefix == PREFIX1 {
                    self.gather.set_alt_right(true);
                } else {
                    self.gather.set_alt_left(true);
                }
            }
            29 => {
                if self.prefix == PREFIX1 {
                    self.gather.set_ctrl_right(true);
                } else {
                    self.gather.set_ctrl_left(true);
                }
            }
            58 => {
                self.gather.set_caps_lock(!self.gather.get_caps_lock());
            }
            70 => {
                self.gather.set_scroll_lock(!self.gather.get_scroll_lock());
            }
            69 => {
                // Numlock oder Pause ?
                if self.gather.get_ctrl_left() {
                    // Pause Taste
                    // Auf alten Tastaturen konnte die Pause-Funktion wohl nur
                    // ueber Ctrl+NumLock erreicht werden. Moderne MF-II Tastaturen
                    // senden daher diese Codekombination, wenn Pause gemeint ist.
                    // Die Pause Taste liefert zwar normalerweise keinen ASCII-
                    // Code, aber Nachgucken schadet auch nicht. In jedem Fall ist
                    // die Taste nun komplett.
                    self.get_ascii_code();
                    done = true;
                } else {
                    // NumLock
                    self.gather.set_num_lock(!self.gather.get_num_lock());
                }
            }

            _ => {
                // alle anderen Tasten
                // ASCII-Codes aus den entsprechenden Tabellen auslesen, fertig.
                self.get_ascii_code();
                done = true;
            }
        }

        // Ein Prefix gilt immer nur fuer den unmittelbar nachfolgenden Code.
        // Also ist es jetzt abgehandelt.
        self.prefix = 0;

        if done {
            return true;
        }
        // Tastaturabfrage abgeschlossen
        else {
            return false;
        }
    }

    /*****************************************************************************
     * Funktion:        get_ascii_code                                           *
     *---------------------------------------------------------------------------*
     * Beschreibung:    Ermittelt anhand von Tabellen aus dem Scancode und den   *
     *                  gesetzten Modifier-Bits den ASCII-Code der Taste.        *
     *****************************************************************************/
    fn get_ascii_code(&mut self) {
        // Sonderfall Scancode 53: Dieser Code wird sowohl von der Minustaste
        // des normalen Tastaturbereichs, als auch von der Divisionstaste des
        // Ziffernblocks gesendet. Damit in beiden Faellen ein Code heraus-
        // kommt, der der Aufschrift entspricht, muss im Falle des Ziffern-
        // blocks eine Umsetzung auf den richtigen Code der Divisionstaste
        // erfolgen.
        if self.code == 53 && self.prefix == PREFIX1 {
            // Divisionstaste des Ziffernblocks
            self.gather.set_ascii('/' as u8);
            self.gather.set_scancode(key::SCAN_DIV);
        }
        // Anhand der Modifierbits muss die richtige Tabelle ausgewaehlt
        // werden. Der Einfachheit halber hat NumLock Vorrang vor Alt,
        // Shift und CapsLock. Fuer Ctrl gibt es keine eigene Tabelle
        else if self.gather.get_num_lock()
            && self.prefix == 0
            && self.code >= 71
            && self.code <= 83
        {
            // Bei eingeschaltetem NumLock und der Betaetigung einer der
            // Tasten des separaten Ziffernblocks (Codes 71-83), sollen
            // nicht die Scancodes der Cursortasten, sondern ASCII- und
            // Scancodes der ensprechenden Zifferntasten geliefert werden.
            // Die Tasten des Cursorblocks (prefix == prefix1) sollen
            // natuerlich weiterhin zur Cursorsteuerung genutzt werden
            // koennen. Sie senden dann uebrigens noch ein Shift, aber das
            // sollte nicht weiter stoeren.
            self.gather
                .set_ascii(ASC_NUM_TAB[(self.code - 71) as usize]);
            self.gather
                .set_scancode(SCAN_NUM_TAB[(self.code - 71) as usize]);
        } else if self.gather.get_alt_right() {
            self.gather.set_ascii(ALT_TAB[self.code as usize]);
            self.gather.set_scancode(self.code);
        } else if self.gather.get_shift() {
            self.gather.set_ascii(SHIFT_TAB[self.code as usize]);
            self.gather.set_scancode(self.code);
        } else if self.gather.get_caps_lock() {
            // Die Umschaltung soll nur bei Buchstaben gelten
            if (self.code >= 16 && self.code <= 26)
                || (self.code >= 30 && self.code <= 40)
                || (self.code >= 44 && self.code <= 50)
            {
                self.gather.set_ascii(SHIFT_TAB[self.code as usize]);
                self.gather.set_scancode(self.code);
            } else {
                self.gather.set_ascii(NORMAL_TAB[self.code as usize]);
                self.gather.set_scancode(self.code);
            }
        } else {
            self.gather.set_ascii(NORMAL_TAB[self.code as usize]);
            self.gather.set_scancode(self.code);
        }
    }

    /*****************************************************************************
     * Funktion:        key_hit_irq                                              *
     *---------------------------------------------------------------------------*
     * Beschreibung:    Diese Methode soll ein Byte von der Tastatur einlesen.   *
     *                                                                           *
     * Rückgabewert:    Wenn der Tastendruck abgeschlossen ist und ein Scancode, *
     *                  sowie gegebenenfalls ein ASCII-Code emittelt werden      *
     *                  konnte, werden diese zurueckgeliefert. Anderenfalls      *
     *                  wird 'invalid' zurueckgegeben.                           *
     *****************************************************************************/
    fn key_hit_irq(&mut self) -> key::Key {
        let invalid: key::Key = Default::default(); // nicht explizit initialisierte Tasten sind ungueltig
        let mut control: u8;

        // warten bis ein Byte abholbereit ist
        loop {
            control = cpu::inb(KBD_CTRL_PORT);
            if (control & KBD_OUTB) != 0 {
                break;
            }
        }

        // Byte einlesen
        self.code = cpu::inb(KBD_DATA_PORT);

        // Auch eine evtl. angeschlossene PS/2 Maus liefert ihre Daten ueber den
        // Tastaturcontroller. In diesem Fall ist zur Kennzeichnung das AUXB-Bit
        // gesetzt.
        if (control & KBD_AUXB) == 0 && self.key_decoded() == true {
            return self.gather;
        }

        return invalid;
    }

    /*****************************************************************************
     * Funktion:        plugin                                                   *
     *---------------------------------------------------------------------------*
     * Beschreibung:    Unterbrechungen fuer die Tastatur erlauben. Ab sofort    *
     *                  wird bei einem Tastendruck die Methode 'trigger'         *
     *                  aufgerufen.                                              *
     *****************************************************************************/
    pub fn plugin() {
        int_dispatcher::register(int_dispatcher::INT_VEC_KEYBOARD, Box::new(KeyboardISR));
        pic::allow(pic::IRQ_KEYBOARD);
    }
}

/*****************************************************************************
 * Implementierung: ISR                                                      *
 *****************************************************************************/
struct KeyboardISR;
impl isr::ISR for KeyboardISR {
    /*****************************************************************************
     * Funktion:        trigger                                                  *
     *---------------------------------------------------------------------------*
     * Beschreibung:    ISR fuer die Tastatur. Wird aufgerufen, wenn die Tastatur*
     *                  eine Unterbrechung ausloest.                             *
     *****************************************************************************/
    fn trigger(&self) {
        let guard = KB.try_lock();
        if guard.is_none() {
            panic!("Could not lock Keyboard");
        }
        let mut kd = guard.unwrap();
        let mut key: key::Key = kd.key_hit_irq();

        if key.valid() {
            let ascii = key.get_ascii() as u8;
            LAST_KEY.store(ascii, Ordering::SeqCst);

            //   cga::setpos(10, 10);
            //    cga::print_byte(k);
        }
    }
}
