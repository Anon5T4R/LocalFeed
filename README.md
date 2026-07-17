# LocalFeed

Leitor de **RSS/Atom** da suíte Local — acompanhe sites e blogs **sem
algoritmo, sem conta, sem tracker**, lendo offline o que já baixou.

## Recursos

**v0.2**
- **Busca** ao vivo nos artigos (título/resumo/feed)
- **Atualização automática** configurável (15/30/60 min, enquanto aberto)

**v0.1**
- **Assinar pelo endereço do site** — a descoberta acha o feed sozinha
  (`<link rel=alternate>` + caminhos comuns tipo `/feed`)
- **Modo leitura**: o artigo completo é baixado e limpo (readability) na
  primeira abertura e fica **cacheado no SQLite** — releitura 100% offline
- **Três visões** (todos / não lidos / favoritos) + lista por feed com
  contadores de não lidos; marcar tudo como lido; favoritos
- **Import/Export OPML** (associação `.opml` registrada) — traga suas
  assinaturas de qualquer leitor e leve embora quando quiser
- Atualizar tudo (F5) com resumo (novos artigos + feeds com erro)
- HTML dos artigos **sanitizado** (scripts/iframes/on* removidos); links
  abrem sempre no navegador do sistema
- Tema claro/escuro/sistema · UI em **PT/EN/ES**

**Rede, honesto:** o LocalFeed é a única exceção "não-offline" da suíte — ele
acessa **apenas** os feeds que você assinou. A *leitura* do que já baixou
funciona sem internet.

## Stack

Tauri 2 + React 19 + Vite + TypeScript no front; Rust no back (`feed-rs`
parser tolerante, `readability`, `reqwest` rustls, `rusqlite` bundled). Sem
telemetria.

## Dev

```bash
npm install
npm run tauri dev   # porta 1466
```

## Release

Tag `vX.Y.Z` → GitHub Actions builda NSIS (Windows) + AppImage (Linux) e
publica a Release. Parte da suíte [Local](https://github.com/Anon5T4R).

## Licença

MIT
