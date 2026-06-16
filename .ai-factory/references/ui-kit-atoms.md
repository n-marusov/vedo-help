# UI-kit атомы (приёмка дизайна)

> Сверено через `pencil` MCP из `ui-kit.lib.pen` (reusable=true).
> Перечислены только компоненты, реализованные в фронтенде как атомы.
> Размеры указаны в соответствии с найденными вариантами в `.pen`.

## Button — `Component/Button`, `Component/Button/Outline`, `Component/Button/Ghost`, `Component/Button/XS`
- `variant`: `primary` (fill `$primary`, текст `$primary-foreground`) | `outline` (stroke `$border`) | `ghost` (stroke `$border`, подложка прозрачная)
- `size`: `md` (padding `8/16`, font 14) | `xs` (padding `4/8`, font `$font-size-3xs`)
- `disabled`, `onClick`, `type`
- slot: `children`, опционально `leftIcon`/`rightIcon`
- шрифт: `IBM Plex Mono`, gap `6` (md) / `4` (xs)

## Input — `Component/Input`
- `value`, `onChange`, `placeholder`
- высота `36`, padding `4/12`, stroke `$input`, fill `$card`, opacity `0.8`
- плейсхолдер: цвет `$muted-foreground`, 13px
- `invalid?: boolean` — стандартный HTML-атрибут применяется как есть (в `.pen` отдельного стиля нет)

## Badge — `Component/Badge`, `Component/Badge/SM`, `Component/Badge/XS`, `Component/Badge/Priority`
- `variant`: `default` (fill `#10b98126`, stroke `#10b9814D`, текст `$primary`) | `priority` (fill `#f9731626`, stroke `#f973164D`, текст `$priority-high`)
- `size`: `md` (padding `2/10`, font `$font-size-2xs`) | `sm` (padding `0/6`, font `$font-size-3xs`) | `xs` (padding `0/4`, font `$font-size-4xs`)
- шрифт: `IBM Plex Mono`

## StatusDot — `Component/StatusDot`
- `status: TaskStatus` (`todo`/`in_progress`/`blocked`/`done`) — цвет кружка берётся из `$status-{status}` (в `.pen` базовый вариант — `$status-done`)
- `withLabel?: boolean` — лейбл в `.pen` присутствует по умолчанию, 12px `IBM Plex Mono`
- ellipse 8×8, gap `6`

## Avatar — в `.pen` нет reusable-компонента
- Реализован только во фронтенде по принципу инициалов из `name`
- `size`: `sm` | `md` | `lg` (значения выбраны фронтендом, т.к. в Pencil образца нет)
- `src?: string`

## Card — `Component/Card`
- дефолтный padding `12`, fill `$card`, stroke `$border`, gap `12`, ширина `300`
- children: `cardTitle` (14/600), `cardDesc` (12, `$muted-foreground`), слот `cardSlot`
- `padding`: оставлен `md` (12) как эталон; `sm`/`lg` — фронтенд-расширение, в `.pen` не зафиксированы
- `tone`: в `.pen` не зафиксирован — фронтенд-расширение

## FormField — `Component/FormField`
- `label` (12/500 `$foreground`) + input-слот (36h, stroke `$input`, fill `$background`) + `error` (`$destructive`, `$font-size-2xs`, `enabled=false` по умолчанию)
- `htmlFor`, `error?`
- `hint?` — в `.pen` не зафиксирован, во фронтенде оставлен как опциональное расширение

## Dialog — в `.pen` есть только `Component/ConfirmDialog`
- реализован на фронте как универсальный `Dialog`: `open`, `onClose`, `title?`, `footer?`
- поведение: закрытие по `Esc` и клику по overlay, focus trap на открытии, портал в `document.body`
- визуально эквивалентен `ConfirmDialog` (padding 24, gap 25, ширина 360)

## Toast — `Component/Toast`
- `variant`: `success` (зафиксирован в `.pen`: fill `#10b98119`, stroke `#10b98166`, icon `circle-check`) | `error` | `info` (варианты наследуют цветовую семантику через `$destructive` / `$muted-foreground` на фронтенде)
- `message: string`
- авто-скрытие через 3500 мс
- провайдер: `ToastProvider` + `useToast()` → `toast.show({...})`
- layout: icon 14 + message 12px + close 12, gap `8`, padding `8/12`, shadow-blur `12`

## Расхождения с Pencil, принятые фронтендом
- Button: в `.pen` нет варианта `danger` — не реализуется.
- Badge: `info`/`success`/`warning` и прочие цветовые варианты не зафиксированы в `.pen`; фронтенд оставляет `default` + `priority`, остальные семантические подсветки делаются через `Toast`/`StatusDot`/CSS-токены.
- Avatar/Dialog.hint/Card.tone — расширения фронтенда, в `.pen` отсутствуют как reusable.
