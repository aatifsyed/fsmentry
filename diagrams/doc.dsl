/// This is documentation for the state machine.
#[derive(Clone)] // these attributes will be passed to
                 // MyStateMachine and the State enum
pub MyStateMachine {
    /// This is a node declaration.
    /// This documentation will be attached to the node.
    ShavingYaks;

    /// This node contains data.
    SweepingHair: usize;

    /// These are edge declarations
    /// This documentation will be shared with each edge.
    ShavingYaks -> SweepingHair -"this is edge-specific documentation"-> Resting;
                        // implicit nodes will be created as appropriate ^
}
