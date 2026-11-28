//! The pause the operator asks for while a job runs, for every front end (DRILLPRO, ROUTEPRO and the modern interface): what state it is in, how the
//! simulated job follows it and what the screen says, in the eight languages. On a real machine the waiting itself is done by
//! [`crate::protocol::execute::run_items_resting`].

use crate::i18n::Lang;
use crate::protocol::execute::Rest;
use crate::runner::{RealRun, RunState};

/// Where a job is with respect to a pause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hold {
    /// No pause: the job goes on.
    Off,
    /// Asked for; the job holds at the end of the hole or path it is on.
    Asked,
    /// Holding with the head clear of the work.
    Paused { spindle_stopped: bool },
    /// The spindle was started again and the head waits for the confirmation.
    SpindleRestarted,
    /// A job on EH3D-SERVER cannot be paused from this program.
    Impossible,
}

/// The pause of a simulated job, which follows the rules of the real one.
#[derive(Debug, Default, Clone)]
pub struct SimHold {
    asked: bool,
    resting: Option<Rest>,
}

impl SimHold {
    pub fn pause(&mut self) {
        if self.resting.is_none() {
            self.asked = true;
        }
    }

    /// Takes back a pause that has not happened yet, or goes on from one that has.
    pub fn resume(&mut self) {
        self.asked = false;
        if matches!(self.resting, Some(Rest::Paused { .. })) {
            self.confirm();
        }
    }

    /// The operator's confirmation of the step the pause is at.
    pub fn confirm(&mut self) {
        self.resting = match self.resting {
            Some(Rest::Paused {
                spindle_stopped: true,
            }) => Some(Rest::SpindleUp),
            _ => None,
        };
    }

    /// The simulated job has just done the item `index` of `items`: true when it holds there.
    pub fn item_done(
        &mut self,
        index: usize,
        safe: &[usize],
        items: usize,
        stops_spindle: bool,
    ) -> bool {
        if self.asked && index + 1 < items && safe.contains(&index) {
            self.asked = false;
            self.resting = Some(Rest::Paused {
                spindle_stopped: stops_spindle,
            });
        }
        self.resting.is_some()
    }

    pub fn resting(&self) -> bool {
        self.resting.is_some()
    }

    pub fn hold(&self) -> Hold {
        match self.resting {
            Some(Rest::Paused { spindle_stopped }) => Hold::Paused { spindle_stopped },
            Some(Rest::SpindleUp) => Hold::SpindleRestarted,
            None if self.asked => Hold::Asked,
            None => Hold::Off,
        }
    }
}

/// The pause of a job: of the real run when there is one, of the simulated one otherwise.
pub fn hold_of(real: Option<&RealRun>, sim: &SimHold) -> Hold {
    let Some(real) = real else {
        return sim.hold();
    };
    match real.state() {
        RunState::Paused { spindle_stopped } => Hold::Paused { spindle_stopped },
        RunState::SpindleRestarted => Hold::SpindleRestarted,
        state if state.finished() => Hold::Off,
        _ if !real.can_pause() => Hold::Impossible,
        _ if real.pause_pending() => Hold::Asked,
        _ => Hold::Off,
    }
}

pub fn pause(real: Option<&RealRun>, sim: &mut SimHold) {
    match real {
        Some(real) => real.pause(),
        None => sim.pause(),
    }
}

pub fn resume(real: Option<&RealRun>, sim: &mut SimHold) {
    match real {
        Some(real) => real.resume(),
        None => sim.resume(),
    }
}

/// The line that says where the pause is; `None` while there is no pause.
pub fn status_text(lang: Lang, hold: Hold) -> Option<&'static str> {
    let table: [&'static str; 8] = match hold {
        Hold::Off => return None,
        Hold::Asked => [
            "Pausing after this hole or path",
            "Pausa al acabar este taladro o tramo",
            "Pausa dopo questo foro o tratto",
            "Pause nach diesem Loch oder Pfad",
            "Pause après ce trou ou ce tracé",
            "Pausa no fim deste furo ou percurso",
            "完成当前孔或路径后暂停",
            "この穴または経路の後で停止します",
        ],
        Hold::Paused {
            spindle_stopped: false,
        } => [
            "PAUSED: head up, spindle running",
            "PAUSA: cabezal arriba, husillo girando",
            "PAUSA: testa alta, mandrino in moto",
            "PAUSE: Kopf oben, Spindel läuft",
            "PAUSE : tête levée, broche en marche",
            "PAUSA: cabeça em cima, fuso a rodar",
            "已暂停:机头抬起,主轴运转中",
            "一時停止:ヘッド上、主軸回転中",
        ],
        Hold::Paused {
            spindle_stopped: true,
        } => [
            "PAUSED: head up, spindle stopped",
            "PAUSA: cabezal arriba, husillo parado",
            "PAUSA: testa alta, mandrino fermo",
            "PAUSE: Kopf oben, Spindel steht",
            "PAUSE : tête levée, broche arrêtée",
            "PAUSA: cabeça em cima, fuso parado",
            "已暂停:机头抬起,主轴已停",
            "一時停止:ヘッド上、主軸停止",
        ],
        Hold::SpindleRestarted => [
            "The spindle runs again. Go on?",
            "El husillo vuelve a girar. ¿Seguir?",
            "Il mandrino gira di nuovo. Avanti?",
            "Spindel läuft wieder. Weiter?",
            "Broche relancée. Continuer ?",
            "O fuso voltou a rodar. Continuar?",
            "主轴已重新启动,继续?",
            "主軸が再始動しました。続行?",
        ],
        Hold::Impossible => [
            "Server job: it keeps running",
            "Trabajo del servidor: sigue en marcha",
            "Lavoro del server: continua",
            "Serverauftrag: läuft weiter",
            "Travail du serveur : il continue",
            "Trabalho do servidor: continua",
            "服务器作业:仍在运行",
            "サーバーの作業は継続します",
        ],
    };
    Some(table[lang as usize])
}

/// The keys of the pause on the menu screens; empty when the job cannot be paused.
pub fn key_text(lang: Lang, hold: Hold) -> &'static str {
    let table: [&'static str; 8] = match hold {
        Hold::Off => [
            "P: pause",
            "P: pausa",
            "P: pausa",
            "P: Pause",
            "P : pause",
            "P: pausa",
            "P:暂停",
            "P:一時停止",
        ],
        Hold::Asked => [
            "P: do not pause",
            "P: no pausar",
            "P: non fermare",
            "P: nicht anhalten",
            "P : ne pas arrêter",
            "P: não parar",
            "P:取消暂停",
            "P:停止を取消",
        ],
        Hold::Paused { .. } | Hold::SpindleRestarted => [
            "Enter: go on",
            "Intro: seguir",
            "Invio: continua",
            "Enter: weiter",
            "Entrée : continuer",
            "Enter: continuar",
            "回车:继续",
            "Enter:続行",
        ],
        Hold::Impossible => return "",
    };
    table[lang as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_simulated_job_holds_only_at_the_end_of_a_hole() {
        let mut sim = SimHold::default();
        sim.pause();
        assert_eq!(sim.hold(), Hold::Asked);
        assert!(!sim.item_done(3, &[5, 8], 10, false));
        assert!(sim.item_done(5, &[5, 8], 10, false));
        assert_eq!(
            sim.hold(),
            Hold::Paused {
                spindle_stopped: false
            }
        );
        sim.resume();
        assert_eq!(sim.hold(), Hold::Off);
    }

    #[test]
    fn a_stopped_spindle_needs_a_second_confirmation() {
        let mut sim = SimHold::default();
        sim.pause();
        assert!(sim.item_done(1, &[1], 4, true));
        sim.resume();
        assert_eq!(sim.hold(), Hold::SpindleRestarted);
        // another resume does not move the head: only the confirmation does
        sim.resume();
        assert_eq!(sim.hold(), Hold::SpindleRestarted);
        sim.confirm();
        assert_eq!(sim.hold(), Hold::Off);
    }

    #[test]
    fn a_pause_can_be_taken_back_and_the_last_item_never_holds() {
        let mut sim = SimHold::default();
        sim.pause();
        sim.resume();
        assert!(!sim.item_done(1, &[1], 4, false));
        sim.pause();
        assert!(!sim.item_done(3, &[3], 4, false));
    }

    #[test]
    fn every_language_has_its_lines() {
        let holds = [
            Hold::Asked,
            Hold::Paused {
                spindle_stopped: false,
            },
            Hold::Paused {
                spindle_stopped: true,
            },
            Hold::SpindleRestarted,
            Hold::Impossible,
        ];
        for lang in Lang::ALL {
            for hold in holds {
                let line = status_text(lang, hold).unwrap();
                // it fits in the window of the classic screens
                assert!(line.chars().count() <= 39, "{lang:?} {line}");
            }
            assert!(status_text(lang, Hold::Off).is_none());
            assert!(!key_text(lang, Hold::Off).is_empty());
        }
    }
}
