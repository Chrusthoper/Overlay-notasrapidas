package main

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/charmbracelet/glamour"
	tea "github.com/charmbracelet/bubbletea"
)

const defaultNotesDir = "notes"

type markdownLoadedMsg struct {
	content string
}

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

		if renderer == nil {
			return markdownLoadedMsg{content: string(raw)}
		}

		rendered, err := renderer.Render(string(raw))
		if err != nil {
			return markdownLoadedMsg{content: string(raw)}
		}

		return markdownLoadedMsg{content: strings.TrimSpace(rendered)}
	}
}
