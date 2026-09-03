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
mouse sobre o morcego produz atenção visual; clicar nele inicia um voo visual
simples e limitado à janela.

Não há física, comportamento complexo ou arte final de animação de asas nesta
etapa.

## BatPet 0.1 — Alive

O estado `HangingIdle` agora combina comportamentos pequenos e independentes:
respiração lenta com variação de ciclo, blink irregular (incluindo double blink
ocasional), atenção suave ao cursor e reajustes ocasionais de postura. Os
olhos usam smoothing, mas o alvo da pupila é contínuo em vez de ficar limitado
às oito direções. Enquanto a nova arte não está presente, o fallback usa apenas
escala ancorada e offsets muito pequenos; os frames revisados substituem isso
por mudanças desenhadas de silhueta.

Ainda não existe uma arte definitiva para olhos fechados. Por enquanto, o blink
é representado ocultando temporariamente as duas pupilas. Um futuro sprite de
pálpebras/olhos fechados poderá substituir essa visualização sem mudar o
agendador ou o estado `BatState`.

A camada `AnimationIntent` → `VisualPose` separa comportamento de frame visual.
O renderer detecta os atlases revisados uma vez no startup e, quando presentes,
usa os frames corporais e a camada de pálpebras; sem eles, o asset atual
continua sendo usado como fallback de uma única célula. A especificação do
primeiro conjunto de arte está em
[`assets/bat/idle/SPRITE_SHEET_SPEC.md`](assets/bat/idle/SPRITE_SHEET_SPEC.md).

## BatPet 0.1.2 — Character Acting

O hover agora é percepção, não comando de voo: a reação de proximidade pode
acontecer enquanto o morcego permanece pendurado; o voo continua reservado ao
clique explícito. A nova camada de acting separa atenção contínua (olhos,
cabeça, corpo e alerta das orelhas), reação `very-near` com antecipação,
recuo e recuperação, e `idle gaze` ocasional quando o cursor não é relevante.

Os frames corporais de atenção ainda dependem do atlas descrito na
especificação. Sem ele, o fallback conserva apenas o deslocamento corporal
mínimo e ancorado, enquanto os olhos mantêm a resposta contínua.

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

O renderer desktop usa Bevy 0.19, uma textura PNG em palette-alpha de 32×32
pixels e nearest-neighbor para preservar o aspecto pixel-art. O arquivo de
arte atual é apenas um asset experimental e pode ser substituído pelo atlas
revisado futuramente.

O projeto foi testado em Linux/KDE/Wayland. A janela solicita always-on-top,
mas Wayland não garante esse recurso de forma portátil; nenhum hack específico
do compositor foi incluído.
