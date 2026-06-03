package main

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/charmbracelet/glamour"
	tea "github.com/charmbracelet/bubbletea"
)

const defaultNotesDir = "notes"

type noteMeta struct {
	duracion int
	progreso int
}

type markdownLoadedMsg struct {
	content  string
	meta     noteMeta
	rawLines []string
}

type appMode int

const (
	ModeNav  appMode = iota
	ModeExec
	ModeEdit
)

func scanNotes(dir string) ([]string, error) {
	entries, err := os.ReadDir(dir)
	if err != nil {
		return nil, err
	}

	var files []string
	for _, e := range entries {
		if e.IsDir() {
			continue
		}
		if strings.HasSuffix(e.Name(), ".md") {
			files = append(files, e.Name())
		}
	}
	return files, nil
}

func newRenderer(width int) *glamour.TermRenderer {
	w := width - 6
	if w < 20 {
		w = 20
	}

	r, err := glamour.NewTermRenderer(
		glamour.WithAutoStyle(),
		glamour.WithWordWrap(w),
	)
	if err != nil {
		return nil
	}
	return r
}

func loadNoteCmd(notesDir, filename string, renderer *glamour.TermRenderer) tea.Cmd {
	return func() tea.Msg {
		path := filepath.Join(notesDir, filename)
		raw, err := os.ReadFile(path)
		if err != nil {
			return markdownLoadedMsg{content: "Error al leer el archivo."}
		}

		rawStr := string(raw)
		meta, body := parseMeta(rawStr)
		rawLines := strings.Split(body, "\n")

		if renderer == nil {
			return markdownLoadedMsg{content: body, meta: meta, rawLines: rawLines}
		}

		rendered, err := renderer.Render(body)
		if err != nil {
			return markdownLoadedMsg{content: body, meta: meta, rawLines: rawLines}
		}

		return markdownLoadedMsg{
			content:  strings.TrimSpace(rendered),
			meta:     meta,
			rawLines: rawLines,
		}
	}
}

func parseMeta(raw string) (noteMeta, string) {
	meta := noteMeta{}

	trimmed := strings.TrimSpace(raw)
	if !strings.HasPrefix(trimmed, "---") {
		return meta, raw
	}

	end := strings.Index(trimmed[3:], "---")
	if end == -1 {
		return meta, raw
	}

	frontMatter := trimmed[3 : end+3]
	body := strings.TrimSpace(trimmed[end+6:])

	for _, line := range strings.Split(frontMatter, "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}

		parts := strings.SplitN(line, ":", 2)
		if len(parts) != 2 {
			continue
		}

		key := strings.TrimSpace(parts[0])
		val := strings.TrimSpace(parts[1])

		switch key {
		case "duracion":
			if v, err := strconv.Atoi(val); err == nil {
				meta.duracion = v
			}
		case "progreso":
			val = strings.TrimSuffix(val, "%")
			if v, err := strconv.Atoi(val); err == nil {
				if v < 0 {
					v = 0
				}
				if v > 100 {
					v = 100
				}
				meta.progreso = v
			}
		}
	}

	return meta, body
}

func toggleTask(notesDir, filename string, lineIndex int) error {
	path := filepath.Join(notesDir, filename)
	raw, err := os.ReadFile(path)
	if err != nil {
		return err
	}

	rawStr := string(raw)
	_, body := parseMeta(rawStr)
	lines := strings.Split(body, "\n")

	if lineIndex < 0 || lineIndex >= len(lines) {
		return nil
	}

	line := lines[lineIndex]
	if strings.Contains(line, "- [ ] ") {
		lines[lineIndex] = strings.Replace(line, "- [ ] ", "- [x] ", 1)
	} else if strings.Contains(line, "- [x] ") {
		lines[lineIndex] = strings.Replace(line, "- [x] ", "- [ ] ", 1)
	}

	newBody := strings.Join(lines, "\n")
	newContent := rebuildFile(rawStr, newBody)
	return os.WriteFile(path, []byte(newContent), 0644)
}

func replaceLine(notesDir, filename string, lineIndex int, newLine string) error {
	path := filepath.Join(notesDir, filename)
	raw, err := os.ReadFile(path)
	if err != nil {
		return err
	}

	rawStr := string(raw)
	_, body := parseMeta(rawStr)
	lines := strings.Split(body, "\n")

	if lineIndex < 0 || lineIndex >= len(lines) {
		return nil
	}

	lines[lineIndex] = newLine
	newBody := strings.Join(lines, "\n")
	newContent := rebuildFile(rawStr, newBody)
	return os.WriteFile(path, []byte(newContent), 0644)
}

func rebuildFile(raw, newBody string) string {
	trimmed := strings.TrimSpace(raw)
	if !strings.HasPrefix(trimmed, "---") {
		return newBody
	}

	end := strings.Index(trimmed[3:], "---")
	if end == -1 {
		return newBody
	}

	frontMatter := trimmed[:end+6]
	return frontMatter + "\n" + newBody + "\n"
}

func countTasks(lines []string) (done, total int) {
	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if strings.HasPrefix(trimmed, "- [") && len(trimmed) > 3 && trimmed[3] == ']' {
			total++
			if trimmed[2] == 'x' {
				done++
			}
		}
	}
	return
}
