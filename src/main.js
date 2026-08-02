let teams = [];
let outputDirectoryPath = "";

async function loadTeams(filePath) {
  const teamsList = document.getElementById("teams-list");
  teamsList.innerHTML = "";

  try {
    teams = await window.__TAURI__.core.invoke("read_team_list", {
      filePath,
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

loadTeams("resources/teams.json");

document.getElementById("browse-teams").addEventListener("click", async () => {
  const selected = await window.__TAURI__.dialog.open({
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });

  if (selected) {
    await loadTeams(selected);
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

function renderTimeSlots(count) {
  const container = document.getElementById("time-slots");

  while (container.children.length > count) {
    container.removeChild(container.lastElementChild);
  }

  while (container.children.length < count) {
    const index = container.children.length;

    const row = document.createElement("div");
    row.className = "field";

    const label = document.createElement("label");
    label.textContent = `Time slot ${index + 1}`;

    const input = document.createElement("input");
    input.type = "time";
    input.className = "time-slot-input";

    row.appendChild(label);
    row.appendChild(input);
    container.appendChild(row);
  }
}

const numberTimeSlotsInput = document.getElementById("number-time-slots");
numberTimeSlotsInput.addEventListener("input", () => {
  const count = Math.max(1, Number(numberTimeSlotsInput.value) || 1);
  renderTimeSlots(count);
});

renderTimeSlots(Number(numberTimeSlotsInput.value));

function collectSeasonInput() {
  const startDate = document.getElementById("start-date").value;
  const endDate = document.getElementById("end-date").value;
  const numberFields = Number(document.getElementById("number-fields").value);
  const gameTimes = Array.from(document.querySelectorAll(".time-slot-input")).map((input) => {
    const [hour, minute] = input.value.split(":").map(Number);
    return { hour, minute };
  });
  const gameDays = Array.from(document.querySelectorAll('input[name="game-days"]:checked')).map(
    (checkbox) => checkbox.value,
  );

  return { startDate, endDate, numberFields, gameTimes, gameDays, teams };
}

document.getElementById("generate-schedule").addEventListener("click", async () => {
  const { startDate, endDate, numberFields, gameTimes, gameDays, teams } = collectSeasonInput();
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
      startDayStr: startDate,
      endDayStr: endDate,
      gameTimes,
      numberFields,
      gameDaysStr: gameDays,
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
