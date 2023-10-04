fsmentry::dsl! {
    pub TrafficLight {
        Red -> RedAmber -> Green -> Amber -> Red;
    }
}

fn cycle_traffic_light() {
    let mut traffic_light = TrafficLight::new(TrafficLightState::Red);
    use TrafficLightEntry as E;
    loop {
        match traffic_light.entry() {
            E::Red(it) => it.red_amber(),
            E::RedAmber(it) => it.green(),
            E::Green(it) => it.amber(),
            E::Amber(it) => it.red(),
        }
    }
}

fn main() {}
