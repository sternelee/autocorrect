import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { setLocale } from './index';

interface LanguageConfig {
	uiLanguage?: string;
}

export async function initI18n() {
	try {
		const config = await invoke<LanguageConfig>('get_config');
		setLocale(config.uiLanguage);
	} catch {
		setLocale('en');
	}

	await listen<{ uiLanguage?: string }>('ui-language-update', (event) => {
		setLocale(event.payload?.uiLanguage);
	});
}
