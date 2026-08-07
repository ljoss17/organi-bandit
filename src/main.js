let teams = [];
let outputDirectoryPath = "";
let currentTeamsFilePath = "resources/teams.json";
let editedTeams = null;

function setTeamsStatus(message, isError) {
  const teamsStatus = document.getElementById("teams-status");
  teamsStatus.textContent = message;
  teamsStatus.classList.remove("success", "error");
  if (message) {
    teamsStatus.classList.add(isError ? "error" : "success");
  }
}

function renderTeamsList() {
  const teamsList = document.getElementById("teams-list");
  teamsList.innerHTML = "";

  if (editedTeams) {
    editedTeams.forEach((team, index) => {
      const item = document.createElement("li");
      item.className = "team-row";

      if (team.isNew) {
        const nameInput = document.createElement("input");
        nameInput.type = "text";
        nameInput.placeholder = "Team name";
        nameInput.value = team.name;
        nameInput.addEventListener("input", () => {
          team.name = nameInput.value;
        });

        const seedInput = document.createElement("input");
        seedInput.type = "number";
        seedInput.min = "0";
        seedInput.step = "1";
        seedInput.placeholder = "Seed";
        seedInput.value = team.seed ?? "";
        seedInput.addEventListener("input", () => {
          team.seed = seedInput.value === "" ? null : Number(seedInput.value);
        });

        item.appendChild(nameInput);
        item.appendChild(seedInput);
      } else {
        const label = document.createElement("span");
        label.textContent = team.seed != null ? `${team.name} ${team.seed}` : team.name;
        item.appendChild(label);
      }

      const removeButton = document.createElement("button");
      removeButton.type = "button";
      removeButton.textContent = "-";
      removeButton.addEventListener("click", () => {
        editedTeams.splice(index, 1);
        renderTeamsList();
      });
      item.appendChild(removeButton);

      teamsList.appendChild(item);
    });
  } else {
    for (const team of teams) {
      const item = document.createElement("li");
      item.textContent = team.seed != null ? `${team.name} ${team.seed}` : team.name;
      teamsList.appendChild(item);
    }
  }
}

async function loadTeams(filePath) {
  try {
    teams = await window.__TAURI__.core.invoke("read_team_list", {
      filePath,
    });
    currentTeamsFilePath = filePath;
    editedTeams = null;
    renderTeamsList();
  } catch (error) {
    document.getElementById("teams-list").textContent = `Failed to load teams: ${error}`;
  }
}

loadTeams(currentTeamsFilePath);

document.getElementById("browse-teams").addEventListener("click", async () => {
  const selected = await window.__TAURI__.dialog.open({
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });

  if (selected) {
    await loadTeams(selected);
  }
});

function setTeamsEditingUI(isEditing) {
  document.getElementById("edit-teams").hidden = isEditing;
  document.getElementById("add-team").hidden = !isEditing;
  document.getElementById("save-teams").hidden = !isEditing;
  document.getElementById("cancel-edit-teams").hidden = !isEditing;
}

document.getElementById("edit-teams").addEventListener("click", () => {
  editedTeams = teams.map((team) => ({ name: team.name, seed: team.seed, isNew: false }));
  setTeamsStatus("", false);
  setTeamsEditingUI(true);
  renderTeamsList();
});

document.getElementById("add-team").addEventListener("click", () => {
  editedTeams.push({ name: "", seed: null, isNew: true });
  renderTeamsList();
});

document.getElementById("cancel-edit-teams").addEventListener("click", async () => {
  editedTeams = null;
  setTeamsStatus("", false);
  setTeamsEditingUI(false);
  await loadTeams(currentTeamsFilePath);
});

document.getElementById("save-teams").addEventListener("click", async () => {
  if (editedTeams.some((team) => team.name.trim() === "")) {
    setTeamsStatus("Every team needs a name.", true);
    return;
  }

  const newTeams = editedTeams.map(({ name, seed }) => ({ name, seed }));

  try {
    await window.__TAURI__.core.invoke("write_team_list", {
      filePath: currentTeamsFilePath,
      newTeams,
    });
    editedTeams = null;
    setTeamsEditingUI(false);
    setTeamsStatus("Saved", false);
    await loadTeams(currentTeamsFilePath);
  } catch (error) {
    setTeamsStatus(String(error), true);
  }
});

document.getElementById("browse-output-folder").addEventListener("click", async () => {
  const selected = await window.__TAURI__.dialog.open({
    directory: true,
  });

  if (selected) {
    outputDirectoryPath = selected;
    document.getElementById("output-folder").value = selected;
  }
});

function readGameTime(fieldPrefix) {
  return {
    hour: Number(document.getElementById(`${fieldPrefix}-hours`).value) || 0,
    minute: Number(document.getElementById(`${fieldPrefix}-minutes`).value) || 0,
  };
}

function collectSeasonInput() {
  const startDate = document.getElementById("start-date").value;
  const numberFields = Number(document.getElementById("number-fields").value);
  const startTime = readGameTime("start-time");
  const timeBetweenGames = readGameTime("time-between-games");
  const startBreak = readGameTime("start-break");
  const endBreak = readGameTime("end-break");
  const gameDays = Array.from(document.querySelectorAll('input[name="game-days"]:checked')).map(
    (checkbox) => checkbox.value,
  );

  return {
    startDate,
    numberFields,
    startTime,
    timeBetweenGames,
    startBreak,
    endBreak,
    gameDays,
    teams,
  };
}

document.getElementById("generate-schedule").addEventListener("click", async () => {
  const {
    startDate,
    numberFields,
    startTime,
    timeBetweenGames,
    startBreak,
    endBreak,
    gameDays,
    teams,
  } = collectSeasonInput();
  const statusMessage = document.getElementById("status-message");
  statusMessage.textContent = "";
  statusMessage.classList.remove("success", "error");

  if (!outputDirectoryPath) {
    statusMessage.textContent = "Please select an output folder before generating.";
    statusMessage.classList.add("error");
    return;
  }

  try {
    const schedule = await window.__TAURI__.core.invoke("tauri_generate_schedule", {
      teams,
      seasonConfig: {
        startDate,
        startTime,
        startBreak,
        endBreak,
        timeBetweenGames,
        numberFields,
        gameDays,
      },
    });
    console.log(schedule);
    await window.__TAURI__.core.invoke("generate_excel_schedule", {
      schedule,
      numberFields,
      outputDirectoryPath,
    });

    statusMessage.textContent = "Success";
    statusMessage.classList.add("success");
  } catch (error) {
    console.error(error);
    statusMessage.textContent = String(error);
    statusMessage.classList.add("error");
  }
});
