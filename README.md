# BatPet

Um companheiro de desktop em pixel-art: um pequeno morcego roxo, pendurado,
curioso e um pouco tímido. Projeto experimental em Rust.

O workspace contém `batpet-desktop` (Bevy 0.19) e o protótipo independente
`batpet` para terminal (`crossterm`).

```bash
cargo run -p batpet-desktop
```

O mouse desperta atenção dentro da janela. O morcego olha primeiro, acompanha
com o rosto e reage à proximidade. Um clique faz um pequeno encolher e espiar,
com retorno ao repouso em 1,45 s. Essa reação substitui o antigo voo visual
indefinido, que deslocava a pose pendurada sem animar asas.

Em repouso há respiração, piscadas irregulares, olhares sustentados, reajustes
de postura e movimentos ocasionais das orelhas. Pausas e cooldowns evitam
sobrepor todos os gestos.

## Direção visual

A textura original e sua paleta foram preservadas. Um rig leve recorta a mesma
textura em suporte, linhas do peito, rosto e pontas das orelhas. Olhos e
pálpebras são pequenos elementos geométricos na mesma grade. Não há atlas
externo pendente nem geração de imagens em runtime.

A janela mantém a escala 8×, nearest-neighbor e transparência. Os movimentos
visuais são quantizados na grade lógica; a respiração insere/remove uma linha,
sem rotação ou escala fracionária. O renderer usa 27 quads e duas texturas
(incluindo a textura branca padrão); a montagem acontece uma única vez.

[Comparação animada antes/depois](docs/visual-review/before-after.gif) ·
[Poses revisadas](docs/visual-review/poses.png) ·
[Decisões e limitações](docs/VISUAL_DIRECTION.md)

## Validação e revisão visual

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo run -p batpet-desktop -- --debug
```

Para repetir a sequência visual no renderer real:

```bash
cargo run -p batpet-desktop -- --review-dir /tmp/batpet-review
cargo run -p batpet-desktop -- --review-dir /tmp/batpet-review-light --review-light
```

Para verificar a grade e o apoio nas capturas (requer Pillow):

```bash
python tools/verify_review.py /tmp/batpet-review
```

O modo de revisão simula atenção à esquerda/direita, proximidade e reação ao
clique, salva capturas a 12 fps e encerra após 13 segundos. Também verifica o
retorno ao estado de repouso. A captura não roda no uso normal. A sequência usa
os timers existentes; pequenas diferenças de quadros podem ocorrer conforme
o tempo de inicialização e o frame rate.

Encerre o aplicativo com `Ctrl+C` ou fechando a janela. Always-on-top e a
posição superior direita são solicitações ao compositor; KDE/Wayland pode
ignorá-las. O cursor continua limitado à área da janela.
