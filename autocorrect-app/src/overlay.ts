import { listen } from '@tauri-apps/api/event';

interface TypoMarker {
	id: string;
	x: number;
	y: number;
	width: number;
	height: number;
	text: string;
}

const container = document.getElementById('markers-container');

function clamp(value: number, min: number, max: number): number {
	return Math.min(Math.max(value, min), max);
}

function resolveMarkerTop(marker: TypoMarker): number {
	if (marker.id.includes('fallback')) {
		return clamp(window.innerHeight - marker.y - 2, 0, Math.max(0, window.innerHeight - 4));
	}

	const topFromTopLeft = marker.y + marker.height - 2;
	const topFromBottomLeft = window.innerHeight - marker.y - 2;

	const topCandidateA = clamp(topFromTopLeft, 0, Math.max(0, window.innerHeight - 4));
	const topCandidateB = clamp(topFromBottomLeft, 0, Math.max(0, window.innerHeight - 4));

	const aDelta = Math.abs(topFromTopLeft - topCandidateA);
	const bDelta = Math.abs(topFromBottomLeft - topCandidateB);
	return aDelta <= bDelta ? topCandidateA : topCandidateB;
}

function renderMarkers(markers: TypoMarker[]): void {
	if (!container) return;
	console.log('[overlay] render markers:', markers.length);
	container.innerHTML = '';

	for (const marker of markers) {
		if (!marker.width || marker.width <= 0) continue;

		const left = `${Math.max(0, marker.x)}px`;
		const width = `${Math.max(2, marker.width)}px`;
		const topA = clamp(marker.y + marker.height - 2, 0, Math.max(0, window.innerHeight - 4));
		const topB = clamp(window.innerHeight - marker.y - 2, 0, Math.max(0, window.innerHeight - 4));
		const isFallback = marker.id.includes('fallback');

		const el = document.createElement('div');
		el.className = 'typo-underline';
		el.style.left = left;
		el.style.top = `${resolveMarkerTop(marker)}px`;
		el.style.width = width;
		container.appendChild(el);

		// Non-fallback markers may still come from inconsistent AX coordinate spaces.
		if (!isFallback && Math.abs(topA - topB) > 6) {
			const mirror = document.createElement('div');
			mirror.className = 'typo-underline';
			mirror.style.left = left;
			mirror.style.top = `${topB}px`;
			mirror.style.width = width;
			mirror.style.opacity = '0.55';
			container.appendChild(mirror);
		}
	}
}

void listen<TypoMarker[]>('update-markers', (event) => {
	renderMarkers(event.payload || []);
});
