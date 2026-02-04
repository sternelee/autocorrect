import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

interface PopupData {
	originalText: string;
	suggestion: string;
	x: number;
	y: number;
}

let currentOriginalText = '';
let currentSuggestion = '';
let isEditing = false;

// DOM Elements
const popup = document.getElementById('popup')!;
const originalTextEl = document.getElementById('originalText')!;
const suggestionTextEl = document.getElementById('suggestionText')!;
const originalEl = document.getElementById('original')!;
const suggestionEl = document.getElementById('suggestion')!;
const editModeEl = document.getElementById('editMode')!;
const editTextarea = document.getElementById('editTextarea') as HTMLTextAreaElement;
const actionsEl = document.getElementById('actions')!;
const editActionsEl = document.getElementById('editActions')!;

// Buttons
const closeBtn = document.getElementById('closeBtn')!;
const rejectBtn = document.getElementById('rejectBtn')!;
const acceptBtn = document.getElementById('acceptBtn')!;
const editBtn = document.getElementById('editBtn')!;
const cancelEditBtn = document.getElementById('cancelEditBtn')!;
const acceptEditBtn = document.getElementById('acceptEditBtn')!;

// Listen for popup-show event from backend
listen<PopupData>('popup-show', (event) => {
	const data = event.payload;
	console.log('Popup show:', data);

	// Update content
	currentOriginalText = data.originalText;
	currentSuggestion = data.suggestion;

	// Update UI
	originalTextEl.textContent = data.originalText;
	suggestionTextEl.textContent = data.suggestion;

	// Show original only if different
	if (data.originalText === data.suggestion) {
		originalEl.classList.add('hidden');
	} else {
		originalEl.classList.remove('hidden');
	}

	// Reset edit mode
	isEditing = false;
	editModeEl.classList.remove('active');
	actionsEl.classList.remove('hidden');
	editActionsEl.classList.add('hidden');

	// Show the window
	getCurrentWindow().show();
	getCurrentWindow().setFocus();
});

// Listen for popup-hide event
listen('popup-hide', () => {
	console.log('Popup hide');
	hidePopup();
});

// Accept suggestion
async function acceptSuggestion() {
	const textToUse = isEditing ? editTextarea.value : currentSuggestion;

	if (!textToUse.trim()) {
		return;
	}

	console.log('Accepting suggestion:', textToUse);

	try {
		// Call accept_suggestion command which will:
		// 1. Set clipboard to the corrected text
		// 2. Hide popup
		// 3. Simulate paste after delay
		await invoke('accept_suggestion', { text: textToUse });
	} catch (error) {
		console.error('Failed to accept suggestion:', error);
	}
}

// Reject suggestion
async function rejectSuggestion() {
	console.log('Rejecting suggestion');

	try {
		await invoke('reject_suggestion');
		// hidePopup() will be called by the backend
	} catch (error) {
		console.error('Failed to reject suggestion:', error);
		hidePopup();
	}
}

function hidePopup() {
	getCurrentWindow().hide();
	resetUI();
}

function resetUI() {
	currentOriginalText = '';
	currentSuggestion = '';
	isEditing = false;
	editModeEl.classList.remove('active');
	actionsEl.classList.remove('hidden');
	editActionsEl.classList.add('hidden');
	originalTextEl.textContent = '-';
	suggestionTextEl.textContent = '-';
	editTextarea.value = '';
}

function startEdit() {
	isEditing = true;
	editTextarea.value = currentSuggestion;
	editModeEl.classList.add('active');
	actionsEl.classList.add('hidden');
	editActionsEl.classList.remove('hidden');
	editTextarea.focus();
}

function cancelEdit() {
	isEditing = false;
	editModeEl.classList.remove('active');
	actionsEl.classList.remove('hidden');
	editActionsEl.classList.add('hidden');
	editTextarea.value = '';
}

// Event listeners
closeBtn.addEventListener('click', rejectSuggestion);
rejectBtn.addEventListener('click', rejectSuggestion);
acceptBtn.addEventListener('click', acceptSuggestion);
editBtn.addEventListener('click', startEdit);
cancelEditBtn.addEventListener('click', cancelEdit);
acceptEditBtn.addEventListener('click', acceptSuggestion);

// Keyboard shortcuts
document.addEventListener('keydown', (e) => {
	if (e.key === 'Enter' && !e.shiftKey && !isEditing) {
		e.preventDefault();
		acceptSuggestion();
	} else if (e.key === 'Escape') {
		e.preventDefault();
		if (isEditing) {
			cancelEdit();
		} else {
			rejectSuggestion();
		}
	}
});

// Auto-resize textarea based on content
editTextarea.addEventListener('input', () => {
	editTextarea.style.height = 'auto';
	editTextarea.style.height = Math.max(60, editTextarea.scrollHeight) + 'px';
});

// Click outside to close (with a small delay to allow button clicks)
let clickTimeout: ReturnType<typeof setTimeout>;
document.addEventListener('click', (e) => {
	const target = e.target as HTMLElement;
	if (!popup.contains(target)) {
		// Ignore clicks that just opened the popup
		clearTimeout(clickTimeout);
		clickTimeout = setTimeout(() => {
			// Only close if the popup is visible
			getCurrentWindow().isVisible().then((isVisible) => {
				if (isVisible) {
					rejectSuggestion();
				}
			});
		}, 100);
	}
});

// Initialize
console.log('Popup page loaded');
