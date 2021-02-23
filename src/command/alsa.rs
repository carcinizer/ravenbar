
use crate::command::{CommandTrait, CommandSharedState};

use std::collections::HashMap;

use alsa::mixer::{Mixer, Selem, SelemId, SelemChannelId};

#[derive(Clone, PartialEq)]
pub enum VolumeChange {
    Percent(isize,f32)
}

#[derive(Clone, PartialEq)]
pub struct ALSAGetVolumeCommand(pub Option<String>);
#[derive(Clone, PartialEq)]
pub struct ALSASetVolumeCommand(pub Option<String>, pub VolumeChange);


struct ALSASingleton {
    mixers: HashMap<String, Option<Mixer>>,
    old_volume: Option<i64>
}

impl CommandTrait for ALSAGetVolumeCommand {
    fn execute(&self, state: &mut CommandSharedState) -> String {
        state.get::<ALSASingleton>(0).get_volume_percent(&self.0)
    }
    fn updated(&self, state: &mut CommandSharedState) -> bool {
        state.get::<ALSASingleton>(0).has_volume_changed(&self.0)
    }
}

impl CommandTrait for ALSASetVolumeCommand {
    fn execute(&self, state: &mut CommandSharedState) -> String {
        state.get::<ALSASingleton>(0).set_volume_percent(&self.0, &self.1);
        String::new()
    }
}


impl VolumeChange {
    pub fn new(s: String) -> Self {
        let (rel, dir) = match s.chars().nth(0) {
            Some('+') => (1, 1f32),
            Some('-') => (1,-1f32),
            _ => (0, 0f32)
        };

        match s.chars().nth_back(0) {
            Some('%') => {
                let change = str::parse::<f32>(&s[rel as usize ..(s.len()-1)]);
                if let Ok(ch) = change {
                    Self::Percent(rel, dir*ch)
                }
                else {
                    eprintln!("Error - Failed to parse volume change percentage '{}'", s);
                    Self::Percent(0,0f32)
                }
            },
            _ => {
                eprintln!("Error - Failed to parse volume change '{}'", s);
                Self::Percent(0,0f32)
            }
        }
    }
}


impl ALSASingleton {
    fn new() -> Self {
        Self {mixers: HashMap::new(), old_volume: None}
    }

    fn with_selem<T,F>(&mut self, card: &Option<String>, f: F) -> Option<T> 
    where F: Fn(Selem) -> T
    {
        let default = String::from("default");
        let name = card.as_ref().unwrap_or(&default);

        let mixer = self.mixers.entry(name.to_string()).or_insert_with(|| {Mixer::new(&name[..], false).ok()});
        mixer.as_ref().and_then(|m| m.find_selem(&SelemId::new("Master", 0)))
            .and_then(|s| Some(f(s)))
    }

    fn get_volumes(&mut self, card: &Option<String>) -> Option<(i64,i64,i64)> {

        self.with_selem(card, |selem| {
            let (vmin, vmax) = selem.get_playback_volume_range();
            selem.get_playback_volume(SelemChannelId::FrontLeft)
                .ok().and_then(|v| Some((v,vmin,vmax)))
        }).flatten()
    }

    fn get_volume_percent(&mut self, card: &Option<String>) -> String {
        
        self.get_volumes(card)
        .and_then(|(v,vmin,vmax)| 
            Some(format!("{}%", ((v-vmin) as f32 / (vmax - vmin) as f32 * 100f32).round()))
        ).unwrap_or(String::from("ERR"))
    }

    fn set_volume_percent(&mut self, card: &Option<String>, vol: &VolumeChange) {

        self.with_selem(card, |selem| {

            #[allow(irrefutable_let_patterns)]
            if let VolumeChange::Percent(rel, change) = vol {

                let (vmin, vmax) = selem.get_playback_volume_range();
                let v = selem.get_playback_volume(SelemChannelId::FrontLeft).unwrap_or(vmin);

                let newvol = (v * *rel as i64 + (*change * (vmax-vmin) as f32/100f32) as i64).max(vmin).min(vmax);
                selem.set_playback_volume_all(newvol)
                    .err().and_then(|x| {eprintln!("Failed to set the new volume: {}", x); Some(())});
            }
        });
    }

    fn has_volume_changed(&mut self, card: &Option<String>) -> bool {
        let new_volume = self.get_volumes(card).and_then(|(v,_,_)| Some(v));
        let updated = new_volume != self.old_volume;

        if updated {self.old_volume = new_volume;}
        updated
    }
}

impl Default for ALSASingleton {
    fn default() -> Self {Self::new()}
}
