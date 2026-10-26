# Todoist Terminal Client Usage Guide

A keyboard-driven Todoist client modelled on Doom Emacs: Vim-style movement, a `SPC` leader key, and a which-key menu. You never need to remember a key. Press a prefix (`SPC`, `g`, `s`, `z`, `d`) and wait a moment, and a menu shows what can follow. `?` shows every binding.

## Layout

- **Sidebar** (toggle with `SPC t s`): Inbox, Today, Upcoming, All, your project tree and labels, each with a task count.
- **List**: the tasks of the current view, grouped by section, project or day, with sub-tasks nested below their parent.
- **Modeline**: mode, view, pending keys and sync state. The line under it shows messages and prompts (the *echo area*). `SPC h m` shows the message log.

Today includes overdue tasks, which appear under an "Overdue" heading. Changes you make appear immediately and are sent to Todoist in the background. If Todoist rejects one, it is rolled back and the reason is shown. The list refreshes itself every 30 seconds (`g r` refreshes now) and the cursor stays on the same task.

## Moving around

| Keys | Action |
|---|---|
| `j` `k`, arrows | next / previous task |
| `g g` / `G` | top / bottom |
| `C-d` / `C-u` | half page down / up |
| `}` / `{` | next / previous section |
| `h` / `l` | collapse (or go to the sidebar) / expand (or go to the list) |
| `TAB`, `z a` | toggle fold of sub-tasks (`z o` open, `z c` close, `z R` open all, `z M` close all) |
| `g t` `g u` `g i` `g a` `g o` | Today, Upcoming, Inbox, All, Overdue |
| `g p`, `SPC SPC` | fuzzy jump to any view, project or label |
| `/` | search all tasks as you type (`Enter` keeps it, `Esc` clears it) |
| `s n` `s d` `s p` | sort: natural, date, priority |

## Working with tasks

| Keys | Action |
|---|---|
| `a` | **quick add** (see below) |
| `A` | add a sub-task to the highlighted task |
| `x` | complete |
| `d d` | delete |
| `u` | undo the last complete / delete / edit |
| `1` `2` `3` `4` | set priority |
| `>` / `<` | postpone a day / move a day earlier |
| `D` | set the due date (natural language; empty clears it) |
| `R` | reschedule: quick presets and a calendar popup (see below) |
| `r` | rename |
| `T` | edit labels |
| `m` | move to another project (fuzzy picker) |
| `e`, `RET` | open the full edit panel |

The same commands live under `SPC m` (`SPC m e`, `SPC m d`, …) if you prefer the leader.

### Quick add

`a` opens a one-line prompt that understands Todoist's quick-add syntax:

```
buy milk tomorrow 5pm p1 #shopping @errands
```

Dates (`tomorrow`, `every monday`, `25 dec`), priority (`p1`–`p4`), project (`#project`) and existing labels (`@label`) are parsed by Todoist. Without `#project`, the task goes to the project you are viewing, or the Inbox. In the Today view it is due today, and in a label view it gets that label.

### Reschedule

`R` (or `SPC m s`) opens a small month calendar on the task's due date (today if it has none). Move the highlighted day with `C-h` / `C-l` (day) and `C-k` / `C-j` (week); `H` / `L` or `C-p` / `C-n` change month. Plain `h j k l` and the arrow keys work too. `Enter` sets the date, `Esc` cancels. The time of day is kept, and `u` undoes it.

Presets set the date straight away: `t` today, `m` tomorrow, `w` next Monday, `n` in a week, `s` the weekend, `x` clears the date. Recurring tasks keep their rule: the schedule simply continues from the date you pick.

### Edit panel

`Tab` / `Shift+Tab` move between Task, Description, Due, Labels and Priority. `Enter` saves (in the description it inserts a new line). `C-c C-c` or `C-s` saves from anywhere, `C-c C-k` aborts, and `Esc` cancels (asking first if you changed something).

## Limitations

- Undoing a delete recreates the task with a new ID, without its sub-tasks or comments.
- Completing a recurring task schedules its next occurrence and can't be undone. `>` and `<` don't work on recurring tasks, so use `D` to edit the schedule.
- Quick add only recognises labels that already exist; an unknown `@label` stays in the task text.
