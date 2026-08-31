# BatPet

BatPet é um projeto de diversão, aprendizado e experimentação com Rust.
Não é um produto pronto: a ideia é explorar a construção de um pequeno
mascote de desktop e testar diferentes arquiteturas de renderização.

## Estado atual

O workspace mantém duas versões separadas:

- `batpet`: protótipo original para terminal com `crossterm`.
- `batpet-desktop`: prova de conceito gráfica com Bevy.

A versão desktop atualmente abre uma janela pequena, transparente e sem
moldura, com um morcego pixel-art pendurado no canto superior direito no
estado `HangingIdle`. Os olhos acompanham o cursor dentro da janela; passar o
mouse sobre o morcego ou clicar nele inicia um voo visual simples e limitado à
janela.

Não há física, comportamento complexo ou animação de asas nesta etapa.

## Como testar

Na raiz do workspace:

```bash
cargo run -p batpet-desktop
```

Para habilitar logs de inicialização e eventos relevantes:

```bash
cargo run -p batpet-desktop -- --debug
```

O aplicativo pode ser encerrado com `Ctrl+C` ou fechando a janela.

O posicionamento no canto superior direito e o always-on-top são solicitações
ao window manager. Em Wayland/KDE o compositor pode ignorar essas propriedades;
o projeto não usa hacks específicos para contornar essa limitação.

## Validação

```bash
cargo fmt --all
cargo check --workspace
cargo test --workspace
```

## Observações

O renderer desktop usa Bevy 0.19, uma textura PNG RGBA de 20×20 pixels e
nearest-neighbor para preservar o aspecto pixel-art. O arquivo de arte atual
é apenas um asset experimental e pode ser substituído futuramente.

O projeto foi testado em Linux/KDE/Wayland. A janela solicita always-on-top,
mas Wayland não garante esse recurso de forma portátil; nenhum hack específico
do compositor foi incluído.
