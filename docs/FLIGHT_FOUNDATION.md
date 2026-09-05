# Flight 0.2 — fundação de movimento

O milestone anterior tratava o morcego como uma criatura pendurada. O sistema
de presença continua sendo o dono do `HangingIdle`; Flight acrescenta apenas a
camada que permite sair do perch, atravessar um pequeno espaço de teste e
retornar à mesma âncora.

## O que existia antes

O estado público só distinguia `HangingIdle` e `Reacting`. O root transform
ficava no ponto de suporte e os sistemas de Alive escreviam respiração,
atenção, olhos e poses nessa entidade. Não havia posição simulada, velocidade,
aceleração ou alvo de voo. Os recortes do rig incluíam as garras na faixa fixa
do sprite, portanto a âncora visual nunca poderia se soltar de forma explícita.

Essa separação é importante: a textura, a paleta, o rig e os sistemas de
percepção não foram redesenhados para criar a mecânica. Flight opera no root e
deixa Alive quieto durante os estados de voo.

## Arquitetura escolhida

`BatState` agora tem quatro estados de voo: `Takeoff`, `Flying`, `Returning` e
`Landing`. As transições são explícitas e executadas por sistemas `OnEnter` e
por atualizações condicionadas ao estado:

```text
HangingIdle
    │ F / --flight-once / --flight-loop
    ▼
Takeoff → Flying → Return → Landing → HangingIdle
```

O componente `FlightMotion` mantém:

- `position`, `velocity` e `acceleration` contínuas;
- `FlightTarget` com posição, raio de chegada e raio de desaceleração;
- limites de velocidade e aceleração;
- tempo da etapa e um marcador de chegada.

`Perch` é o ponto de extensão para futuros locais de pouso. Nesta etapa ele
guarda a âncora do hanging e um `approach_offset`; não conhece janelas,
monitores, colisões ou geometria do desktop.

O steering é um `arrive` simples. A direção aponta para o alvo, a velocidade
desejada diminui dentro do raio de desaceleração e também é limitada pela
velocidade de frenagem possível com a aceleração disponível. A diferença entre
velocidade desejada e velocidade atual vira aceleração limitada. A integração
usa delta time com clamp de passo para evitar saltos quando o processo é
interrompido. Valores não finitos são substituídos por um fallback seguro.

A simulação permanece contínua. Apenas a escrita no `Transform` é quantizada na
grade de 8 pixels, preservando nearest-neighbor e pixel-perfect rendering sem
transformar a física em uma sequência de teletransportes.

## Takeoff, retorno e garras

No `Takeoff`, a posição começa na âncora, zera velocidade e aceleração e recebe
um alvo curto ao lado e dentro da área visível. A faixa superior do rig, que contém a
relação visual das garras com o corpo, fica visível por `TAKEOFF_SUPPORT_HOLD`;
depois é escondida durante `Flying` e `Returning`.

`Flying` usa um alvo local diagonal apenas para tornar a continuidade
observável. Quando chega, permanece em repouso por `FLIGHT_DWELL`; isso deixa a
etapa existir como estado e evita que a saída pareça uma única curva contínua.
`Returning` aponta primeiro para o ponto de aproximação do perch. `Landing`
segue então para a âncora exata; as garras reaparecem nos últimos pixels da
aproximação. Ao entrar em `HangingIdle`, `finish_landing` fixa posição e
velocidade na âncora, devolvendo o controle aos sistemas Alive.

## Como observar

Use o modo de desenvolvimento:

```bash
cargo run -p batpet-desktop -- --debug --flight-loop
```

O comando inicia um ciclo após um segundo e repete-o depois do pouso. Para um
ciclo único:

```bash
cargo run -p batpet-desktop -- --debug --flight-once
```

Com a janela focada, `F` também dispara o ciclo. Os logs mostram cada entrada
de estado, incluindo o retorno a `HangingIdle`. A sequência não altera o modo
normal e não move o cursor.

Para guardar quadros do renderer durante um ciclo, use a revisão opt-in:

```bash
cargo run -p batpet-desktop -- \
  --review-dir /tmp/batpet-flight-review --review-flight --flight-once
```

Esse modo captura a saída, a pausa em voo, a aproximação e o pouso a 12 fps e
encerra depois de confirmar o retorno ao repouso.

[Quadros observados no renderer](visual-review/flight-cycle.png)

## Testes e limites

Os testes de `pet::flight` rodam sem renderer e cobrem aceleração limitada,
velocidade máxima, convergência a partir de lados diferentes, preservação da
grade, alvos de aproximação e valores finitos. `cargo fmt --all`,
`cargo check --workspace` e `cargo test --workspace` são a validação de
integração.

Esta etapa deliberadamente não tenta vender o movimento como voo acabado. O
sprite ainda usa a pose base durante o deslocamento; não há batimento de asas,
inclinação, banking, impulso de takeoff ou secondary motion. Os alvos continuam
locais e determinísticos, há um único perch e a janela ainda é o único espaço
conhecido. Esses aspectos pertencem à próxima iteração visual depois que a
fundação mecânica for observada e aprovada.
