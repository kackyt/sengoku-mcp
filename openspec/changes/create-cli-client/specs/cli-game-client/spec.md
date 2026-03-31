## ADDED Requirements

### Requirement: CLI Launch and Daimyo Selection
The system SHALL present a title screen and allow the user to select a Daimyo to play as.

#### Scenario: Launching the CLI
- **WHEN** user starts the `cli` application
- **THEN** system displays the title screen and a list of playable Daimyos.

#### Scenario: Selecting a Daimyo
- **WHEN** user selects a Daimyo from the list and presses Enter
- **THEN** system initializes the game with the selected Daimyo as the player, and transitions to the Domestic Mode (内政モード) screen for that Daimyo's turn.

### Requirement: Domestic Mode Operations
The CLI SHALL provide a UI for Domestic Mode, displaying current resources and allowing commands (War, Sell Rice, Buy Rice, Develop, etc.) to be executed.

#### Scenario: Viewing Resources
- **WHEN** user is in Domestic Mode
- **THEN** system displays the player's Gold, Rice, Troops, Population, Kokudaka, Towns, and Loyalty.

#### Scenario: Executing a Domestic Command
- **WHEN** user selects a domestic command (e.g., "開墾") and enters the required amount of Gold
- **THEN** system passes the command to the `DomesticUseCase`, updates the game state, and reflects the new resource values on the screen.

### Requirement: War Mode Operations
The CLI SHALL allow the user to transition to War Mode and handle combat against adjacent territories.

#### Scenario: Initiating War
- **WHEN** user selects the "戦争" command in Domestic Mode and chooses an adjacent enemy territory
- **THEN** system transitions to the War Mode screen, displaying army information and battle tactics (通常, 奇襲, 火計, etc.).

#### Scenario: Executing Battle Tactics
- **WHEN** user selects a battle tactic during their turn in War Mode
- **THEN** system passes the tactic using `BattleUseCase`, calculates the combat result, and updates army states (Troops, Morale, Food).

### Requirement: Turn Progression
The CLI SHALL process turn progression, including NPC (CPU) moves and seasonal events.

#### Scenario: NPC Turn Execution
- **WHEN** player ends their turn or a CPU Daimyo's turn begins
- **THEN** system automatically executes the CPU's move via `TurnProgressionUseCase` and displays the result abruptly, then continues to the next Daimyo's turn until all turns complete the season.
