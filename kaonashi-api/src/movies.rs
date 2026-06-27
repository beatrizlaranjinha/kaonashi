pub const MOVIES_70S: [&str; 8] = [
    "Taxi Driver",
    "The Godfather",
    "Apocalypse Now",
    "Barry Lyndon",
    "Suspiria",
    "Eraserhead",
    "A Clockwork Orange",
    "Rocky",
];

pub const MOVIES_80S: [&str; 8] = [
    "The Shining",
    "Scarface",
    "Blue Velvet",
    "Paris, Texas",
    "Cinema Paradiso",
    "After Hours",
    "Grave of the Fireflies",
    "Blade Runner",
];

pub const MOVIES_90S: [&str; 8] = [
    "Fight Club",
    "Pulp Fiction",
    "Se7en",
    "Goodfellas",
    "Eyes Wide Shut",
    "Casino",
    "Fallen Angels",
    "Reservoir Dogs",
];

pub const MOVIES_2000S: [&str; 8] = [
    "There Will Be Blood",
    "Inglourious Basterds",
    "Mulholland Drive",
    "American Psycho",
    "The Dark Knight",
    "In the Mood for Love",
    "Oldboy",
    "Howl's Moving Castle",
];

pub const MOVIES_2010S: [&str; 8] = [
    "Whiplash",
    "Interstellar",
    "Once Upon a Time in Hollywood",
    "Shutter Island",
    "Django Unchained",
    "The Hateful Eight",
    "Phantom Thread",
    "Lucky",
];

pub const MOVIES_2020S: [&str; 8] = [
    "Marty Supreme",
    "Oppenheimer",
    "Perfect Days",
    "The Holdovers",
    "Anora",
    "Hamnet",
    "Dune: Part Two",
    "Poor Things",
];

pub fn movies_decades(decade_id: u8) -> Option<Vec<String>> {
    match decade_id {
        0 => Some(MOVIES_70S.iter().map(|x| x.to_string()).collect()), //percorre a lista de filmes , pega em cada x e tranforma e str para string, junta todos numa nova lista
        1 => Some(MOVIES_80S.iter().map(|x| x.to_string()).collect()),
        2 => Some(MOVIES_90S.iter().map(|x| x.to_string()).collect()),
        3 => Some(MOVIES_2000S.iter().map(|x| x.to_string()).collect()),
        4 => Some(MOVIES_2010S.iter().map(|x| x.to_string()).collect()),
        5 => Some(MOVIES_2020S.iter().map(|x| x.to_string()).collect()),
        _ => None,
    }
}
