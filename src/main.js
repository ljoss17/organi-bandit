let teams = [];

async function loadTeams() {
  const teamsList = document.getElementById("teams-list");

  try {
    teams = await window.__TAURI__.core.invoke("read_team_list", {
      filePath: "resources/teams.json",
    });

    for (const team of teams) {
      const item = document.createElement("li");
      item.textContent = team.seed != null ? `${team.name} ${team.seed}` : team.name;
      teamsList.appendChild(item);
    }
  } catch (error) {
    teamsList.textContent = `Failed to load teams: ${error}`;
  }
}

loadTeams();

function collectSeasonInput() {
  const startDate = document.getElementById("start-date").value;
  const endDate = document.getElementById("end-date").value;
  const maxGamesPerDay = Number(document.getElementById("max-games-per-day").value);
  const gameDays = Array.from(document.querySelectorAll('input[name="game-days"]:checked')).map(
    (checkbox) => checkbox.value,
  );

  return { startDate, endDate, maxGamesPerDay, gameDays, teams };
}

document.getElementById("generate-schedule").addEventListener("click", async () => {
  const { startDate, endDate, maxGamesPerDay, gameDays, teams } = collectSeasonInput();

  try {
    const schedule = await window.__TAURI__.core.invoke("tauri_generate_schedule", {
      teams,
      startDayStr: startDate,
      endDayStr: endDate,
      maxGamesPerDay,
      gameDaysStr: gameDays,
    });
    console.log(schedule);
  } catch (error) {
    console.error(error);
  }
});
