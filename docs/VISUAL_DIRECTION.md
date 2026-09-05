# Direção visual — presença antes de quantidade

A intenção é um morceguinho tranquilo, curioso e tímido. O repouso deve ser
agradável por si só. Um gesto deve sugerir que ele percebeu algo e depois
voltou a ficar confortável.

## Diagnóstico observado

A versão original foi executada antes das alterações. O desenho já tinha
identidade: corpo roxo, olhos grandes, orelhas rosadas e silhueta pendurada.
Porém, a respiração deformava a textura inteira em escala fracionária;
o blink apagava pupilas sem fechar os olhos; atenção e microgestos dependiam
de atlases inexistentes. Além disso, `resolve_visual_pose` não copiava os
canais contínuos de atenção. O clique iniciava deslocamento e rotação sem
batidas de asas nem retorno ao repouso.

## O que mudou e por quê

- **Olhos:** sockets arredondados, pupilas cheias com brilho único e pálpebras
  de fechamento real. A primeira tentativa com a pupila antiga em cruz ficou
  vidrada; a revisão trouxe um olhar mais cheio e legível. O brilho acompanha
  a pupila. As diagonais são limitadas para não escapar do contorno arredondado.
- **Apoio e respiração:** garras e parte superior imóveis. O peito ganha uma
  linha durante a inspiração, e o rosto acompanha um pixel abaixo. A primeira
  tentativa esticava sete linhas em oito e produzia pixels desiguais; foi
  substituída por scanlines inteiras. Há mais repouso que expansão.
- **Cabeça e orelhas:** atenção contínua convertida em deslocamentos de até um
  pixel lógico. As pontas das orelhas têm movimento próprio. A primeira montagem
  revelou frestas; uma linha de sobreposição na raiz mantém o encaixe.
- **Timing:** piscadas de 160–230 ms, com variação lenta ocasional de 280–360 ms;
  olhares espontâneos sustentados por 1–2 s. Intervalos e cooldowns preservam
  silêncio entre os gestos. Os olhares foram ampliados para permanecerem
  visíveis após a quantização.
- **Clique:** antecipação, encolher com olhos fechados, espiar com a orelha e
  recuperação em 1,45 s. É uma mudança deliberada de interação: o antigo voo
  provisório foi substituído por uma reação coerente com a pose pendurada.
- **Sistema:** o rig utiliza recortes da textura original e formas simples;
  o renderer não depende mais de folhas de poses ausentes. Os PNGs originais
  continuam intactos. O protótipo de terminal permanece independente.

As decisões de maior impacto são o fechamento verdadeiro dos olhos, o apoio
fixo e o gesto que retorna ao repouso. Elas alteram a leitura da intenção do
personagem, sem depender de aumentar o número de eventos.

## Evidência e comparação

A versão original de HEAD foi extraída para um diretório temporário e executada
separadamente com a mesma sequência de estímulos. A comparação usa capturas do
renderer Bevy, não uma animação ilustrativa reconstruída:

- [Antes/depois animado](visual-review/before-after.gif)
- [Comparação parada](visual-review/before-after.png)
- [Repouso, atenção, proximidade e clique](visual-review/poses.png)

Foram inspecionados 144 quadros da versão revisada. Todos os blocos 8×8 eram
uniformes: zero violações da grade nas capturas. O suporte permanece idêntico
entre os quadros. A revisão também foi executada sobre fundo claro.

O custo visual é pequeno e fixo: 27 quads, criados no startup, recortes de uma
textura 32×32 e a textura branca padrão. Não há alocação de assets, leitura de
disco ou rasterização de imagens a cada frame no modo normal. Não foi feito
benchmark comparativo de CPU/GPU; essa é uma avaliação estrutural de custo.

## Limitações restantes

A fonte continua sendo a arte experimental de 32×32. Há ruído de pixels no
peito e alguma ambiguidade anatômica entre asas e corpo. O rig melhora atuação,
mas não substitui uma revisão autoral completa dessa anatomia.

O movimento de um pixel a 8× é intencionalmente escalonado; não tem a suavidade
de uma animação subpixel. Existem apenas duas etapas de pálpebra. A pequena
reação ao clique não pretende ser uma simulação de carinho ou um sistema de
humor. Não há voo animado nesta versão.

O mouse só é percebido dentro da janela. Always-on-top e posicionamento ainda
dependem do compositor, especialmente em Wayland. Capturas do framebuffer
usam fundo preto quando a janela é transparente; a transparência do desktop
foi conferida na execução normal.
