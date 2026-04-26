use anchor_lang::prelude::*;

declare_id!("89JzYW3Jpk7xAn8Mbgx3JLfH5xmN67Tp23VEUam7Q7eE");

const MAX_PROPOSALS: usize = 5;
const MAX_PROPOSALS_NAME: usize = 32;
const MAX_VOTERS: usize = 20;

//equivalente ao contrato na solidity, at least i hope
#[program]
pub mod owner_project {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>, proposals_names: Vec<String>) -> Result<()> {
        require!(
            //macro do ancor
            !proposals_names.is_empty(), //garante que existem propostas
            VotingError::NoProposals     //lança o erro
        );

        let voting_state = &mut ctx.accounts.voting_state; //vai buscar a conta onde vamos buscar dados
        let chairperson = ctx.accounts.user.key(); //

        voting_state.chairperson = chairperson;
        voting_state.proposals = Vec::new();
        voting_state.voters = Vec::new();
        voting_state.chairperson_vote_index = None;

        for name in proposal_names {
            //nome da proposta
            require!(
                name.as_bytes().len() <= MAX_PROPOSAL_NAME,
                VotingError::ProposalNameTooLong
            );
            //adiconar uma nova proposta a lista de proposta
            voting_state.proposals.push(Proposal {
                name,
                vote_count: 0,
            });
        }
        //chairperson passa a ser autoizado a votar
        voting_state.voters.push(Voter {
            address: chairperson,
            allowed_to_vote: true,
            voted: false,
            vote_index: None,
        });

        Ok(())
    }
    //o chairperson chama alguém para votar
    pub fn give_right_to_vote(ctx: Context<OnlyChairperson>, voter: Pubkey) -> Result<()> {
        let voting_state = &mut ctx.accounts.voting_state; //estado da votação que vai ser alterado

        match voter_found {
            Some(existing_voter) => {
                //o voter já existe
                require!(!existing_voter.voted, VotingError::AlreadyVoted); //já votou
                require!(
                    !existing_voter.allowed_to_vote, //ou não tem permissão
                    VotingError::AlreadyAllowedToVote
                );

                existing_voter.allowed_to_vote = true; //dar direito a votar
            }

            None => {
                require!(
                    // não existe , verifica se não ultrapassa o maximo
                    voting_state.voters.len() < MAX_VOTERS,
                    VotingError::TooManyVoters
                );

                voting_state.voters.push(Voter {
                    //cria um novo voter
                    address: voter,
                    allowed_to_vote: true,
                    voted: false,
                    vote_index: None,
                });
            }
        }

        Ok(())
    }

    pub fn vote(ctx: Context<Vote>, proposal_index: u8) -> Result<()> {
        let voting_state = &mut ctx.accounts.voting_state; //estado da votação que vai ser alterado
        let voter_address = ctx.accounts.voter.key(); //endereço de quem está a votar

        let proposal_index_usize = proposal_index as usize; //os vetores usam usize como índice

        require!(
            proposal_index_usize < voting_state.proposals.len(),
            VotingError::InvalidProposal
        ); //verifica se o índice está dentro do número de propostas

        let voter = match voting_state
            .voters
            .iter_mut()
            .find(|v| v.address == voter_address) //procura o voter com este endereço
        {
            Some(v) => v, //se encontrou usa o voter

            None => {
                return Err(VotingError::VoterNotFound.into()); //se não encontrou devolve erro
            }
        };

        require!(voter.allowed_to_vote, VotingError::NotAllowedToVote);
        require!(!voter.voted, VotingError::AlreadyVoted);

        Ok(())
    }

    //equivalente as variaveis de contrato i think
    #[account]
    pub struct VotingState {
        pub owner: Pubkey,
        pub value: u64,
    }

    pub struct Voter {
        pub address: Pubkey,
        pub allowed_to_vote: bool,
        pub voted: bool,
        pub vote_index: Option<u8>,
    }

    #[derive(Accounts)]
    pub struct Initialize<'info> {
        #[account(init, payer = user, space = 8 + 32 + 8)]
        //criar uma nova conta com init, sapce memoria necessaria
        pub voting_state: Account<'info, VotingState>, //onde ficam guardados os dados
        #[account(mut)]
        pub user: Signer<'info>, //quem chama assina
        pub system_program: Program<'info, System>, // programa inter da solana para criar contas
    }

    #[derive(Accounts)]
    pub struct OnlyOwner<'info> {
        #[account(mut, has_one = owner)]
        pub voting_state: Account<'info, VotingState>,
        pub owner: Signer<'info>,
    }

    #[error_code]
    pub enum VotingError {
        #[msg("No proposals provided")]
        NoProposals,

        #[msg("Too many proposals")]
        TooManyProposals,

        #[msg("Name to long")]
        ProposalNameTooLong,

        #[msg("Too many voters.")]
        TooManyVoters,

        #[msg("Already voted..")]
        AlreadyVoted,

        #[msg("Already ready to vote")]
        AlreadyAllowedToVote,

        #[msg("This voter does not exist.")]
        VoterNotFound,

        #[msg("Not allowed to vote.")]
        NotAllowedToVote,

        #[msg("Invalid.")]
        InvalidProposal,
    }
}
