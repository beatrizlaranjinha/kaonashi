use anchor_lang::prelude::*;

declare_id!("89JzYW3Jpk7xAn8Mbgx3JLfH5xmN67Tp23VEUam7Q7eE");

const MAX_PROPOSALS: usize = 5;
const MAX_PROPOSAL_NAME: usize = 32;
const MAX_VOTERS: usize = 20;

// equivalente ao contrato na solidity, at least i hope
#[program]
pub mod owner_project {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, proposals_names: Vec<String>) -> Result<()> {
        require!(
            // macro do anchor
            !proposals_names.is_empty(), // garante que existem propostas
            VotingError::NoProposals     // lança o erro
        );

        require!(
            proposals_names.len() <= MAX_PROPOSALS,
            VotingError::TooManyProposals
        );

        let voting_state = &mut ctx.accounts.voting_state; // vai buscar a conta onde vamos guardar dados
        let chairperson = ctx.accounts.user.key(); // quem chama a função passa a ser o chairperson

        voting_state.chairperson = chairperson;
        voting_state.proposals = Vec::new();
        voting_state.voters = Vec::new();
        voting_state.chairperson_vote_index = None;

        for name in proposals_names {
            // nome da proposta
            require!(
                name.as_bytes().len() <= MAX_PROPOSAL_NAME,
                VotingError::ProposalNameTooLong
            );

            // adicionar uma nova proposta a lista de propostas
            voting_state.proposals.push(Proposal {
                name,
                vote_count: 0,
            });
        }
        // chairperson passa a ser autorizado a votar
        let proposals_len = voting_state.proposals.len();

        voting_state.voters.push(Voter {
            address: chairperson,
            allowed_to_vote: true,
            voted: false,
            vote_index: None,
            vote_vector: vec![0; proposals_len],
        });

        Ok(())
    }

    // o chairperson chama alguém para votar
    pub fn give_right_to_vote(ctx: Context<OnlyChairperson>, voter: Pubkey) -> Result<()> {
        let voting_state = &mut ctx.accounts.voting_state; // estado da votação que vai ser alterado

        let voter_found = voting_state.voters.iter_mut().find(|v| v.address == voter); // procura se já existe alguém com este endereço

        match voter_found {
            Some(existing_voter) => {
                // o voter já existe
                require!(!existing_voter.voted, VotingError::AlreadyVoted); // já votou
                require!(
                    !existing_voter.allowed_to_vote, // ou já tem permissão
                    VotingError::AlreadyAllowedToVote
                );

                existing_voter.allowed_to_vote = true; // dar direito a votar
            }

            None => {
                require!(
                    // não existe, verifica se não ultrapassa o máximo
                    voting_state.voters.len() < MAX_VOTERS,
                    VotingError::TooManyVoters
                );

                let proposals_len = voting_state.proposals.len();

                voting_state.voters.push(Voter {
                    address: voter,
                    allowed_to_vote: true,
                    voted: false,
                    vote_index: None,
                    vote_vector: vec![0; proposals_len],
                });
            }
        }

        Ok(())
    }

    pub fn vote(ctx: Context<Vote>, proposal_index: u8) -> Result<()> {
        let voting_state = &mut ctx.accounts.voting_state; // estado da votação que vai ser alterado
        let voter_address = ctx.accounts.voter.key(); // endereço de quem está a votar

        let proposal_index_usize = proposal_index as usize; // os vetores usam usize como índice

        require!(
            proposal_index_usize < voting_state.proposals.len(),
            VotingError::InvalidProposal
        ); // verifica se o índice está dentro do número de propostas

        // guardar antes para evitar conflito de borrow
        let proposals_len = voting_state.proposals.len();

        let voter = match voting_state
            .voters
            .iter_mut()
            .find(|v| v.address == voter_address) // procura o voter com este endereço
        {
            Some(v) => v,
            None => return Err(VotingError::VoterNotFound.into()),
        };

        require!(voter.allowed_to_vote, VotingError::NotAllowedToVote);
        require!(!voter.voted, VotingError::AlreadyVoted);

        // mete 1 na opção escolhida e deixa as outras a zero
        let mut vote_vector = vec![0; proposals_len];
        vote_vector[proposal_index_usize] = 1;

        voter.voted = true; // já votou
        voter.vote_index = Some(proposal_index); // guarda a opção
        voter.vote_vector = vote_vector; // guarda o vetor

        voting_state.proposals[proposal_index_usize].vote_count += 1; // adiciona o voto a proposta

        if voter_address == voting_state.chairperson {
            // verifica se o chairperson votou
            voting_state.chairperson_vote_index = Some(proposal_index); // usa para desempate
        }

        Ok(())
    }

    pub fn change_chairperson(
        ctx: Context<OnlyChairperson>,
        new_chairperson: Pubkey,
    ) -> Result<()> {
        let voting_state = &mut ctx.accounts.voting_state;

        require!(
            voting_state.voters.iter().all(|v| !v.voted), // percorre todos os voters e verifica se ng votou para poder alterar o chairperson
            VotingError::VotingAlreadyStarted
        );

        voting_state.chairperson = new_chairperson;

        Ok(())
    }
}

// equivalente as variaveis de contrato i think
#[account]
pub struct VotingState {
    pub chairperson: Pubkey,
    pub proposals: Vec<Proposal>,
    pub voters: Vec<Voter>,
    pub chairperson_vote_index: Option<u8>,
}

impl VotingState {
    pub const SPACE: usize =
        // anchor, chairperson, vetor proposals, espaço das propostas, tamanho do vetor voters, espaço dos voters, option do chairperson
        8 + 32 + 4 + MAX_PROPOSALS * Proposal::SPACE + 4 + MAX_VOTERS * Voter::SPACE + 1 + 1;

    // começa com zero votos, percorre tudo e guarda o maior
    pub fn max_votes(&self) -> u64 {
        self.proposals
            .iter()
            .fold(0, |acc, p| acc.max(p.vote_count))
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = VotingState::SPACE)]
    // criar uma nova conta com init, space memoria necessaria
    pub voting_state: Account<'info, VotingState>, // onde ficam guardados os dados

    #[account(mut)]
    pub user: Signer<'info>, // quem chama assina

    pub system_program: Program<'info, System>, // programa interno da solana para criar contas
}

#[derive(Accounts)]
pub struct OnlyChairperson<'info> {
    #[account(mut, has_one = chairperson)]
    pub voting_state: Account<'info, VotingState>,

    pub chairperson: Signer<'info>,
}

#[derive(Accounts)]
pub struct Vote<'info> {
    #[account(mut)]
    pub voting_state: Account<'info, VotingState>,

    pub voter: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Proposal {
    pub name: String,
    pub vote_count: u64,
}

impl Proposal {
    pub const SPACE: usize = 4 + MAX_PROPOSAL_NAME + 8;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct Voter {
    pub address: Pubkey,
    pub allowed_to_vote: bool,
    pub voted: bool,
    pub vote_index: Option<u8>,
    pub vote_vector: Vec<u8>,
}

impl Voter {
    pub const SPACE: usize = 32 + 1 + 1 + 1 + 1 + 4 + MAX_PROPOSALS;
}

#[error_code]
pub enum VotingError {
    #[msg("No proposals provided")]
    NoProposals,

    #[msg("Too many proposals")]
    TooManyProposals,

    #[msg("Name too long")]
    ProposalNameTooLong,

    #[msg("Too many voters.")]
    TooManyVoters,

    #[msg("Already voted.")]
    AlreadyVoted,

    #[msg("Already allowed to vote")]
    AlreadyAllowedToVote,

    #[msg("This voter does not exist.")]
    VoterNotFound,

    #[msg("Not allowed to vote.")]
    NotAllowedToVote,

    #[msg("Invalid proposal.")]
    InvalidProposal,

    #[msg("Voting already started.")]
    VotingAlreadyStarted,
}
