import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface TypoSuggestion {
  typo: string;
  suggestions: string[];
  line: number;
  col: number;
}

interface PopupData {
  originalText: string;
  suggestion: string;
  x: number;
  y: number;
  typos?: TypoSuggestion[];
  offset?: number;
  charLength?: number;
}

let currentOriginalText = "";
let currentSuggestion = "";
let currentTypos: TypoSuggestion[] = [];
let currentOffset: number | null = null;
let currentCharLength: number | null = null;
let isEditing = false;

// DOM Elements
const popup = document.getElementById("popup")!;
const originalTextEl = document.getElementById("originalText")!;
const suggestionTextEl = document.getElementById("suggestionText")!;
const typosListEl = document.getElementById("typosList")!;
const originalEl = document.getElementById("original")!;
const suggestionEl = document.getElementById("suggestion")!;
const typosEl = document.getElementById("typos")!;
const editModeEl = document.getElementById("editMode")!;
const editTextarea = document.getElementById(
  "editTextarea",
) as HTMLTextAreaElement;
const actionsEl = document.getElementById("actions")!;
const editActionsEl = document.getElementById("editActions")!;

// Buttons
const closeBtn = document.getElementById("closeBtn")!;
const rejectBtn = document.getElementById("rejectBtn")!;
const acceptBtn = document.getElementById("acceptBtn")!;
const editBtn = document.getElementById("editBtn")!;
const cancelEditBtn = document.getElementById("cancelEditBtn")!;
const acceptEditBtn = document.getElementById("acceptEditBtn")!;

// Listen for popup-show event from backend
listen<PopupData>("popup-show", (event) => {
  const data = event.payload;
  console.log("Popup show:", data);
  console.log("Typos received:", data.typos);
  console.log("Typos type:", typeof data.typos);
  console.log("Typos is array?", Array.isArray(data.typos));

  // Update content
  currentOriginalText = data.originalText;
  currentSuggestion = data.suggestion;
  currentTypos = data.typos || [];
  currentOffset = data.offset ?? null;
  currentCharLength = data.charLength ?? null;

  console.log("currentTypos:", currentTypos);
  console.log("currentTypos length:", currentTypos.length);

  // Update UI
  originalTextEl.textContent = data.originalText;
  suggestionTextEl.textContent = data.suggestion;

  // Show original only if different
  if (data.originalText === data.suggestion) {
    originalEl.classList.add("hidden");
  } else {
    originalEl.classList.remove("hidden");
  }

  // Render typos list
  console.log("About to render typos list with:", currentTypos);
  renderTyposList(currentTypos);

  // Reset edit mode
  isEditing = false;
  editModeEl.classList.remove("active");
  actionsEl.classList.remove("hidden");
  editActionsEl.classList.add("hidden");

  // Window is shown by the backend via orderFrontRegardless (macOS).
  // No-op here to avoid accidentally activating the app.
});

// Listen for popup-hide event
listen("popup-hide", () => {
  hidePopup();
});

// Accept suggestion
async function acceptSuggestion() {
  const textToUse = isEditing ? editTextarea.value : currentSuggestion;

  if (!textToUse.trim()) {
    return;
  }

  console.log("Accepting suggestion:", textToUse);

  try {
    // Call accept_suggestion command which will:
    // 1. Set clipboard to the corrected text
    // 2. Hide popup
    // 3. Simulate paste after delay
    // Tauri v2 serialises JS camelCase → Rust snake_case automatically, so
    // the Rust parameter `char_length` must be sent as `charLength` from JS.
    await invoke("accept_suggestion", {
      text: textToUse,
      offset: currentOffset,
      charLength: currentCharLength,
    });
  } catch (error) {
    console.error("Failed to accept suggestion:", error);
  }
}

// Reject suggestion
async function rejectSuggestion() {

  try {
    await invoke("reject_suggestion");
    // hidePopup() will be called by the backend
  } catch (error) {
    console.error("Failed to reject suggestion:", error);
    hidePopup();
  }
}

function hidePopup() {
  getCurrentWindow().hide();
  resetUI();
}

function resetUI() {
  currentOriginalText = "";
  currentSuggestion = "";
  currentTypos = [];
  isEditing = false;
  editModeEl.classList.remove("active");
  actionsEl.classList.remove("hidden");
  editActionsEl.classList.add("hidden");
  originalTextEl.textContent = "-";
  suggestionTextEl.textContent = "-";
  editTextarea.value = "";
  typosListEl.innerHTML = "";
  typosEl.classList.add("hidden");
}

function renderTyposList(typos: TypoSuggestion[]) {
  console.log("renderTyposList called with:", typos);
  console.log("typos length:", typos?.length);

  if (!typos || typos.length === 0) {
    console.log("No typos, hiding typos section");
    typosEl.classList.add("hidden");
    return;
  }

  console.log("Showing typos section for", typos.length, "typos");
  typosEl.classList.remove("hidden");
  typosListEl.innerHTML = "";

  typos.forEach((typo, index) => {
    console.log(`Rendering typo ${index}:`, typo);
    const typoItem = document.createElement("div");
    typoItem.className = "typo-item";

    const typoError = document.createElement("div");
    typoError.className = "typo-error";
    typoError.innerHTML = `
			<span class="typo-word">${escapeHtml(typo.typo)}</span>
			<span class="typo-location">Line ${typo.line}, Col ${typo.col}</span>
		`;

    const suggestionsList = document.createElement("div");
    suggestionsList.className = "typo-suggestions";

    typo.suggestions.slice(0, 3).forEach((suggestion, index) => {
      const suggestionBtn = document.createElement("button");
      suggestionBtn.className = "typo-suggestion-btn";
      suggestionBtn.textContent = suggestion;
      suggestionBtn.title = `Replace "${typo.typo}" with "${suggestion}"`;
      suggestionBtn.onclick = () => applyTypoSuggestion(typo.typo, suggestion);
      suggestionsList.appendChild(suggestionBtn);
    });

    // Add "Add to Custom" button if there are suggestions
    if (typo.suggestions.length > 0) {
      const addCustomBtn = document.createElement("button");
      addCustomBtn.className = "typo-custom-btn";
      addCustomBtn.textContent = "+ Add to Custom";
      addCustomBtn.title = `Add "${typo.typo} → ${typo.suggestions[0]}" to custom corrections`;
      addCustomBtn.onclick = () =>
        addToCustomCorrections(typo.typo, typo.suggestions[0]);
      suggestionsList.appendChild(addCustomBtn);
    }

    typoItem.appendChild(typoError);
    typoItem.appendChild(suggestionsList);
    typosListEl.appendChild(typoItem);
  });

  console.log(
    "Finished rendering typos, typosListEl children:",
    typosListEl.children.length,
  );
}

function applyTypoSuggestion(typo: string, suggestion: string) {
  // Replace the typo in the current suggestion
  const regex = new RegExp(`\\b${escapeRegex(typo)}\\b`, "gi");
  currentSuggestion = currentSuggestion.replace(regex, suggestion);
  suggestionTextEl.textContent = currentSuggestion;

  // Remove the typo from the list
  currentTypos = currentTypos.filter((t) => t.typo !== typo);
  renderTyposList(currentTypos);
}

async function addToCustomCorrections(typo: string, correction: string) {
  try {
    await invoke("add_custom_correction", { typo, correction });
    console.log(`Added custom correction: ${typo} → ${correction}`);

    // Show feedback - we could add a notification system, but for now just remove it from the list
    // as if it was applied
    currentTypos = currentTypos.filter((t) => t.typo !== typo);
    renderTyposList(currentTypos);

    // Optional: Show a brief success message
    showNotification(`Added "${typo} → ${correction}" to custom corrections.`);
  } catch (error) {
    console.error("Failed to add custom correction:", error);
    showNotification(`Failed to add custom correction: ${error}`, true);
  }
}

function showNotification(message: string, isError = false) {
  // Create a notification element
  const notification = document.createElement("div");
  notification.className = isError
    ? "notification notification-error"
    : "notification notification-success";
  notification.textContent = message;
  document.body.appendChild(notification);

  // Remove after 3 seconds
  setTimeout(() => {
    notification.remove();
  }, 3000);
}

function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}

function escapeRegex(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function startEdit() {
  isEditing = true;
  editTextarea.value = currentSuggestion;
  editModeEl.classList.add("active");
  actionsEl.classList.add("hidden");
  editActionsEl.classList.remove("hidden");
  editTextarea.focus();
}

function cancelEdit() {
  isEditing = false;
  editModeEl.classList.remove("active");
  actionsEl.classList.remove("hidden");
  editActionsEl.classList.add("hidden");
  editTextarea.value = "";
}

// Event listeners
closeBtn.addEventListener("click", rejectSuggestion);
rejectBtn.addEventListener("click", rejectSuggestion);
acceptBtn.addEventListener("click", acceptSuggestion);
editBtn.addEventListener("click", startEdit);
cancelEditBtn.addEventListener("click", cancelEdit);
acceptEditBtn.addEventListener("click", acceptSuggestion);

// Keyboard shortcuts
document.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey && !isEditing) {
    e.preventDefault();
    acceptSuggestion();
  } else if (e.key === "Escape") {
    e.preventDefault();
    if (isEditing) {
      cancelEdit();
    } else {
      rejectSuggestion();
    }
  }
});

// Auto-resize textarea based on content
editTextarea.addEventListener("input", () => {
  editTextarea.style.height = "auto";
  editTextarea.style.height = Math.max(60, editTextarea.scrollHeight) + "px";
});

// Click outside to close (with a small delay to allow button clicks)
let clickTimeout: ReturnType<typeof setTimeout>;
document.addEventListener("click", (e) => {
  const target = e.target as HTMLElement;
  if (!popup.contains(target)) {
    // Ignore clicks that just opened the popup
    clearTimeout(clickTimeout);
    clickTimeout = setTimeout(() => {
      // Only close if the popup is visible
      getCurrentWindow()
        .isVisible()
        .then((isVisible) => {
          if (isVisible) {
            rejectSuggestion();
          }
        });
    }, 100);
  }
});

// Initialize
console.log("Popup page loaded");
