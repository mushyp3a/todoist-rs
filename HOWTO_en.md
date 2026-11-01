# Todoist Terminal Client Usage Guide

A keyboard-driven Todoist client laid out and keyed like lazygit: bordered panels, single-key commands, and a hint bar at the bottom. `?` (or `x`) lists every binding. Press a prefix (`SPC`, `d`, `g`, `s`, `p`) and wait a moment, and a menu shows what can follow.

## Layout

- **Views panel** (`1` focuses it, `|` toggles it, `TAB` switches panels): Inbox, Today, Upcoming, All, your project tree and labels, each with a task count.
- **Tasks panel** (`2`): the tasks of the current view, grouped by section, project or day, with sub-tasks nested below their parent.
- **Bottom line**: key hints, plus messages and prompts when active, and sync state at the right. `@` shows the message log.

Today includes overdue tasks, which appear under an "Overdue" heading. Changes you make appear immediately and are sent to Todoist in the background. If Todoist rejects one, it is rolled back and the reason is shown. The list refreshes itself every 30 seconds (`R` refreshes now) and the cursor stays on the same task.

## Moving around

| Keys | Action |
|---|---|
| `j` `k`, arrows | next / previous task |
| `<` / `>` | top / bottom |
| `C-d` / `C-u` | half page down / up |
| `}` / `{` | next / previous section |
| `h` / `l` | collapse (or go to the sidebar) / expand (or go to the list) |
| `TAB` | next panel |
| `1` `2` | focus Views / Tasks panel |
| `f` | toggle fold of sub-tasks (`-` collapse all, `+` expand all) |
| `g t` `g u` `g i` `g a` `g o` | Today, Upcoming, Inbox, All, Overdue |
| `g p`, `C-p` | fuzzy jump to any view, project or label |
| `/` | filter all tasks as you type (`Enter` keeps it, `Esc` clears it) |
| `o` | cycle sort order |
| `s n` `s d` `s p` | sort: natural, date, priority |

## Working with tasks

| Keys | Action |
|---|---|
| `a` | **quick add** (see below) |
| `A` | add a sub-task to the highlighted task |
| `SPC SPC` | complete (press twice) |
| `d d` | delete (press twice) |
| `z` | undo the last complete / delete / edit |
| `p 1`–`p 4` | set priority |
| `]` / `[` | postpone a day / move a day earlier |
| `D` | set the due date (natural language; empty clears it) |
| `t` | reschedule: quick presets and a calendar popup (see below) |
| `r` | rename |
| `T` | edit labels |
| `m` | move to another project (fuzzy picker) |
| `e`, `RET` | open the full edit panel |

### Quick add

`a` opens a one-line prompt that understands Todoist's quick-add syntax:

```
buy milk tomorrow 5pm p1 #shopping @errands
```

Dates (`tomorrow`, `every monday`, `25 dec`), priority (`p1`–`p4`), project (`#project`) and existing labels (`@label`) are parsed by Todoist. Without `#project`, the task goes to the project you are viewing, or the Inbox. In the Today view it is due today, and in a label view it gets that label.

### Reschedule

`t` opens a small month calendar on the task's due date (today if it has none). Move the highlighted day with `C-h` / `C-l` (day) and `C-k` / `C-j` (week); `H` / `L` or `C-p` / `C-n` change month. Plain `h j k l` and the arrow keys work too. `Enter` sets the date, `Esc` cancels. The time of day is kept, and `z` undoes it.

Presets set the date straight away: `t` today, `m` tomorrow, `w` next Monday, `n` in a week, `s` the weekend, `x` clears the date. Recurring tasks keep their rule: the schedule simply continues from the date you pick.

### Edit panel

`Tab` / `Shift+Tab` move between Task, Description, Due, Labels and Priority. `Enter` saves (in the description it inserts a new line). `C-c C-c` or `C-s` saves from anywhere, `C-c C-k` aborts, and `Esc` cancels (asking first if you changed something).

## Limitations

- Undoing a delete recreates the task with a new ID, without its sub-tasks or comments.
- Completing a recurring task schedules its next occurrence and can't be undone. `]` and `[` don't work on recurring tasks, so use `D` to edit the schedule.
- Quick add only recognises labels that already exist; an unknown `@label` stays in the task text.
