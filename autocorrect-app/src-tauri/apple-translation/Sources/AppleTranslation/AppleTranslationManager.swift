import Foundation
import Translation

final class AppleTranslationManager {
    static let shared = AppleTranslationManager()

    private init() {}

    func translateBlocking(sourceText: String, sourceLang: String?, targetLang: String) -> String {
        guard #available(macOS 26.0, *) else {
            return "[error] Apple Translation requires macOS 26.0 or later"
        }

        let semaphore = DispatchSemaphore(value: 0)
        var output = ""

        Task {
            do {
                output = try await self.translate(
                    sourceText: sourceText,
                    sourceLang: sourceLang,
                    targetLang: targetLang
                )
            } catch let translationError as TranslationError {
                output = "[error] Apple Translation: \(translationError.localizedDescription)"
            } catch {
                output = "[error] \(error.localizedDescription)"
            }
            semaphore.signal()
        }

        semaphore.wait()
        return output
    }

    @available(macOS 26.0, *)
    private func translate(sourceText: String, sourceLang: String?, targetLang: String) async throws -> String {
        guard let targetLanguage = Self.language(targetLang) else {
            throw NSError(domain: "dev.autocorrect.app.apple-translation", code: 2, userInfo: [
                NSLocalizedDescriptionKey: "Apple Translation requires a valid target language code (e.g. en, zh-Hans, ja).",
            ])
        }

        let sourceLanguage: Locale.Language? = sourceLang.flatMap(Self.language)
            ?? Locale.preferredLanguages.compactMap(Self.language).first

        guard let sourceLanguage else {
            throw NSError(domain: "dev.autocorrect.app.apple-translation", code: 5, userInfo: [
                NSLocalizedDescriptionKey: "Apple Translation could not determine the source language.",
            ])
        }

        let availability = LanguageAvailability()
        let status = await availability.status(from: sourceLanguage, to: targetLanguage)
        switch status {
        case .unsupported:
            throw NSError(domain: "dev.autocorrect.app.apple-translation", code: 3, userInfo: [
                NSLocalizedDescriptionKey: "Apple Translation does not support this source/target language pair.",
            ])
        case .supported:
            try await ensureLanguageInstalled(
                availability: availability,
                source: sourceLanguage,
                target: targetLanguage
            )
        case .installed:
            break
        @unknown default:
            break
        }

        let session = TranslationSession(installedSource: sourceLanguage, target: targetLanguage)
        let response = try await session.translate(sourceText)
        return response.targetText
    }

    @available(macOS 26.0, *)
    private func ensureLanguageInstalled(
        availability: LanguageAvailability,
        source: Locale.Language,
        target: Locale.Language
    ) async throws {
        let session = TranslationSession(installedSource: source, target: target)
        if session.canRequestDownloads {
            try? await session.prepareTranslation()
        }

        let pollIntervalNanos: UInt64 = 500_000_000
        let maxAttempts = 60
        for _ in 0..<maxAttempts {
            let status = await availability.status(from: source, to: target)
            if status == .installed {
                return
            }
            try? await Task.sleep(nanoseconds: pollIntervalNanos)
        }

        throw NSError(domain: "dev.autocorrect.app.apple-translation", code: 4, userInfo: [
            NSLocalizedDescriptionKey: "Source language is not installed for Apple Translation. Please install it in System Settings > Apple Intelligence & Siri > Language.",
        ])
    }

    @available(macOS 26.0, *)
    private static func language(_ code: String) -> Locale.Language? {
        let trimmed = code.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty || trimmed.caseInsensitiveCompare("auto") == .orderedSame {
            return nil
        }
        return Locale.Language(identifier: trimmed)
    }
}
