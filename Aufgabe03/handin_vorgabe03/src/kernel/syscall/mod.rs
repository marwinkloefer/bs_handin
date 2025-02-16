/*****************************************************************************
 *                                                                           *
 *                  s y s c a l l                                            *
 *                                                                           *
 *---------------------------------------------------------------------------*
 * Beschreibung:    Funktionstabelle fuer alle Systemaufrufe sowie Macros    *
 *                  um Systemaufrufe mit 1 - 5 Parametern zu realisieren.    *
 *                  Die Uebergabe der Parameter erfolgt in Registern,        *
 *                  gemaess der System V ABI fuer AMD64, siehe hier:         *
 *                  https://www.uclibc.org/docs/psABI-x86_64.pdf             *
 *                                                                           *
 * Autor:           Stefan Lankes, RWTH Aachen                               *
 *    Erweitert von Michael Schoettner, 13.09.2023                           *
 *****************************************************************************/

pub mod user_api;
pub mod syscall_dispatcher;
pub mod kfuncs;
