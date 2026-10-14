<!-- eidos-i18n: source=docs/guide/fallout4.md sha=474124b57d5bbd3ef319fce7399039bddab4249d -->

# Fallout 4 através do Eidos

Fallout 4 não precisa de nenhuma opção de inicialização especial, nenhum executável
renomeado e nenhum script embrulhador. Vale dizer isso com clareza, porque todo
outro guia de Linux para F4SE afirma o contrário - e o conselho deles quebra na
próxima atualização do Steam.

## A opção de inicialização

```
~/.local/bin/eidos-gui %command%
```

O alvo de inicialização do Steam para Fallout 4 é `Fallout4Launcher.exe`, nunca
`Fallout4.exe`, então fazer o script extender rodar é na verdade a pergunta "como
faço o Steam iniciar outro programa". As respostas usuais reescrevem `%command%` em
bash:

```
bash -c 'exec "${@/Fallout4Launcher.exe/f4se_loader.exe}"' -- %command%
```

ou copiam `f4se_loader.exe` por cima de `Fallout4Launcher.exe`, que o Steam restaura
caladamente a cada atualização do jogo - depois disso você está jogando sem F4SE e
nada avisa.

O Eidos faz a troca sozinho, a partir do descritor do jogo: substitui o lançador por
`f4se_loader.exe` quando há um instalado, recai em `Fallout4.exe` quando não há, e
**avisa você** quando teve de recair. Um jogo que inicia com todos os mods de F4SE
inertes é pior que um jogo que não inicia.

Há um segundo motivo para nunca executar o lançador: ele revarre `Data` e reescreve
`plugins.txt`, desfazendo a ordem de carga recém-implantada. O Eidos nunca o executa.

## Do que o Eidos cuida por você

| | |
|---|---|
| Invalidação de arquivos | `Fallout4Custom.ini` recebe `[Archive]` `bInvalidateOlderFiles=1` e um `sResourceDataDirsFinal=` vazio, as duas chaves que permitem que arquivos soltos fora de `Data` sejam sequer vistos. Escrito no perfil, não na pasta do jogo. |
| Ordem de carga | `plugins.txt` no formato de asterisco que o Fallout 4 usa (`*` marca ativo), com `Fallout4.ccc` respeitado para os plugins implícitos do Creation Club |
| LOOT | A ordenação funciona igual à do Skyrim - `eidos sort <instance>` busca a masterlist de `fallout4` |
| Saves | Saves `.fos` e seus cosaves `.f4se` são listados, copiados e mantidos por perfil; o painel de detalhes lê a tabela de plugins do próprio save, então um save que precisa de um plugin desativado avisa antes de você carregá-lo |
| Mods root | Tudo que um mod traz ao lado do executável (o próprio F4SE, ENB, um `dxvk.conf`) cai lá pelo mesmo mecanismo `Root/` que o Skyrim usa |

## A questão das versões

Fallout 4 não é mais o jogo congelado que foi entre 2019 e 2024. Em agosto de 2026
há três ramos vivos, e uma DLL de mod construída para um não carrega em outro:

| Ramo | Versão | F4SE |
|---|---|---|
| Clássico ("old-gen") | 1.10.163 | 0.6.23 |
| Next-gen | 1.10.984 | 0.7.2 |
| Anniversary / Creations | 1.11.137 → 1.11.240 | 0.7.4 → 0.7.9 |

Duas consequências que vale conhecer antes de montar uma lista de mods:

- **Confira o que você realmente tem.** Pastas `Creations/` e `Mods/` na raiz do
  jogo significam que você está na linha 1.11.x. O painel de detalhes de um save no
  Eidos também mostra a build que o escreveu - o Fallout grava isso no save, e o
  Eidos exibe como "Game build".
- **Um patch recém-saído não é um bom dia para começar.** O F4SE costuma sair um ou
  dois dias depois de uma atualização da Bethesda, mas a *Address Library for F4SE
  Plugins* - por onde a maioria dos mods DLL resolve seus deslocamentos - segue o
  próprio calendário. Entre os dois, a metade DLL do ecossistema está no chão. Mods
  sem DLL (texturas, malhas, plugins) não são afetados.

Assim que sua pilha funcionar, desligue as atualizações automáticas do Steam para
Fallout 4 (Propriedades → Atualizações → "Só atualizar este jogo quando eu o
iniciar"), ou o próximo patch quebrará toda DLL que você instalou.

## Nota de hardware: destroços de armas travam em NVIDIA

O efeito de destroços de armas do Fallout 4 roda sobre o NVIDIA FleX, um derivado do
PhysX que a NVIDIA parou de dar suporte depois da geração Pascal. Em qualquer placa
Turing ou mais nova - GTX 16, RTX 20 até RTX 50 - ele derruba o jogo. É um bug do
jogo, sem relação com Linux, Proton ou Eidos.

Dois consertos, qualquer um serve: desligue "Weapon Debris" nas configurações do
jogo, ou instale o *Weapon Debris Crash Fix* (Nexus 48078), que desativa a colisão
dos fragmentos em vez do efeito.

## Se algo parece errado

A lista geral está em [troubleshooting.pt-BR.md](troubleshooting.md); a
primeira pergunta específica do Fallout é sempre *qual executável realmente
iniciou*. O Eidos escreve o comando de inicialização completo no log de execução da
instância, então:

```sh
grep '# command:' <instance>/logs/run-*.log | tail -1
```

Se nomear `f4se_loader.exe`, a troca aconteceu. Se nomear `Fallout4Launcher.exe`, o
F4SE não está instalado onde o Eidos consegue achar - o lugar dele é ao lado do
executável do jogo, o que numa instalação gerenciada significa o diretório `Root/`
de um mod (ou a própria pasta do jogo, instalado à mão).
