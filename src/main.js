async function loadTeams() {
  const teamsList = document.getElementById("teams-list");

  try {
    const teams = await window.__TAURI__.core.invoke("read_team_list", {
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
