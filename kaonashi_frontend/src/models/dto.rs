use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProposalDto {
    pub index: u8,
    pub name: String,
    pub emoji: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BallotDto {
    pub title: String,
    pub proposals: Vec<ProposalDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoteRequestDto {
    pub proposal_index: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoteResponseDto {
    pub success: bool,
    pub message: String,
}
