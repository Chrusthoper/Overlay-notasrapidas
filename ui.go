package main

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
)

var (
	panelStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("63")).
			Padding(0, 1)

	titleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("212")).
			Background(lipgloss.Color("57")).
			Padding(0, 1)

	selectedStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("212")).
			Bold(true)

	normalStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("252"))

	cursorStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("212"))

	placeholderStyle = lipgloss.NewStyle().
				Foreground(lipgloss.Color("246")).
				Italic(true)
)

func renderView(m model) string {
	if m.width == 0 {
		return "Cargando..."
	}

	leftWidth := m.width / 3
	rightWidth := m.width - leftWidth - 4
	panelHeight := m.height - 2

	leftPanel := renderFileList(m, leftWidth, panelHeight)
	rightPanel := renderPreview(m, rightWidth, panelHeight)

	return lipgloss.JoinHorizontal(lipgloss.Top, leftPanel, rightPanel)
}

func renderFileList(m model, width, height int) string {
	var b strings.Builder

	header := titleStyle.Render(" 📂 Notas ")
	b.WriteString(header)
	b.WriteString("\n\n")

	visibleHeight := height - 4
	if visibleHeight < 1 {
		visibleHeight = 10
	}

	if len(m.files) == 0 {
		b.WriteString(placeholderStyle.Render("No se encontraron notas."))
	} else {
		for i, name := range m.files {
			if i < m.scrollOffset() || i >= m.scrollOffset()+visibleHeight {
				continue
			}

			cursor := "  "
			style := normalStyle

			if i == m.cursor {
				cursor = cursorStyle.Render("❯ ")
				style = selectedStyle
			}

			displayName := strings.TrimSuffix(name, ".md")
			line := fmt.Sprintf("%s%s", cursor, style.Render(displayName))
			b.WriteString(line)
			b.WriteString("\n")
		}
	}

	content := b.String()
	return panelStyle.
		Width(width - 2).
		Height(height).
		Render(content)
}

func renderPreview(m model, width, height int) string {
	var content string

	if len(m.files) == 0 {
		content = placeholderStyle.Render("\n  Selecciona una nota para ver su contenido...")
	} else if m.loading {
		content = placeholderStyle.Render("\n  ⣾ Cargando...")
	} else if m.renderedContent == "" {
		content = placeholderStyle.Render("\n  Selecciona una nota para ver su contenido...")
	} else {
		content = m.renderedContent
	}

	header := titleStyle.Render(" 📄 Vista Previa ")
	full := header + "\n\n" + content

	return panelStyle.
		Width(width - 2).
		Height(height).
		Render(full)
}

func (m model) scrollOffset() int {
	visibleHeight := m.height - 6
	if visibleHeight < 1 {
		visibleHeight = 10
	}

	if m.cursor < visibleHeight {
		return 0
	}
	return m.cursor - visibleHeight + 1
}
