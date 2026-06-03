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

	barFilledStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("82"))

	barEmptyStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("240"))

	barLabelStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("252"))

	barPercentStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("82"))

	lineHighlightStyle = lipgloss.NewStyle().
				Background(lipgloss.Color("57")).
				Foreground(lipgloss.Color("230"))

	lineActiveStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("63"))

	statusBarStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("230")).
			Background(lipgloss.Color("57"))

	statusModeStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("230")).
			Background(lipgloss.Color("125"))

	statusFileStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("252")).
			Background(lipgloss.Color("57"))

	statusTasksStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("82")).
			Background(lipgloss.Color("57"))
)

func renderView(m model) string {
	if m.width == 0 {
		return "Cargando..."
	}

	statusHeight := 1
	availableHeight := m.height - statusHeight

	topHeight := availableHeight * 7 / 10
	bottomHeight := availableHeight - topHeight

	leftWidth := m.width / 3
	rightWidth := m.width - leftWidth - 4

	leftPanel := renderFileList(m, leftWidth, topHeight)
	rightPanel := renderPreview(m, rightWidth, topHeight)
	topRow := lipgloss.JoinHorizontal(lipgloss.Top, leftPanel, rightPanel)

	bottomPanel := renderPlanning(m, m.width, bottomHeight)

	mainContent := lipgloss.JoinVertical(lipgloss.Left, topRow, bottomPanel)
	statusBar := renderStatusBar(m, m.width)

	return lipgloss.JoinVertical(lipgloss.Left, mainContent, statusBar)
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
	} else if m.mode == ModeExec || m.mode == ModeEdit {
		content = renderRawLines(m, width)
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

func renderRawLines(m model, width int) string {
	if len(m.rawLines) == 0 {
		return placeholderStyle.Render("\n  Sin contenido.")
	}

	var b strings.Builder
	for i, line := range m.rawLines {
		if i == m.lineCursor {
			if m.mode == ModeEdit {
				m.textInput.Width = width - 8
				editLine := fmt.Sprintf("  ❯ %s", m.textInput.View())
				b.WriteString(lineHighlightStyle.Render(editLine))
			} else {
				prefix := "  ❯ "
				b.WriteString(lineHighlightStyle.Render(prefix + line))
			}
		} else {
			prefix := "    "
			b.WriteString(lineActiveStyle.Render(prefix + line))
		}
		if i < len(m.rawLines)-1 {
			b.WriteString("\n")
		}
	}

	return b.String()
}

func renderPlanning(m model, width, height int) string {
	header := titleStyle.Render(" 📊 Planificación / Línea de Tiempo ")

	var content string
	if m.meta.duracion == 0 && m.meta.progreso == 0 {
		content = placeholderStyle.Render("\n  Esta nota no tiene metadatos de planificación.")
		content += placeholderStyle.Render("\n  Agrega front matter con duracion y progreso.")
	} else {
		barWidth := width - 30
		if barWidth < 10 {
			barWidth = 10
		}
		if barWidth > 40 {
			barWidth = 40
		}

		filled := m.meta.progreso * barWidth / 100
		empty := barWidth - filled

		bar := barFilledStyle.Render(strings.Repeat("█", filled)) +
			barEmptyStyle.Render(strings.Repeat("░", empty))

		duracionText := barLabelStyle.Render(fmt.Sprintf("Duración: %d días", m.meta.duracion))
		percentText := barPercentStyle.Render(fmt.Sprintf("%d%%", m.meta.progreso))

		content = fmt.Sprintf("\n  %s  %s  %s", duracionText, bar, percentText)
	}

	full := header + "\n" + content

	return panelStyle.
		Width(width - 2).
		Height(height).
		Render(full)
}

func renderStatusBar(m model, width int) string {
	modeText := statusModeStyle.Render(fmt.Sprintf(" %s ", m.modeString()))
	fileText := statusFileStyle.Render(fmt.Sprintf(" %s ", m.activeFileName()))

	done, total := m.taskCounts()
	tasksText := statusTasksStyle.Render(fmt.Sprintf(" ✓ %d/%d ", done, total))

	usedWidth := len(m.modeString()) + 3 + len(m.activeFileName()) + 2 + len(fmt.Sprintf(" ✓ %d/%d ", done, total))
	padding := width - usedWidth
	if padding < 0 {
		padding = 0
	}

	return statusBarStyle.Render(modeText + fileText + strings.Repeat(" ", padding) + tasksText)
}

func (m model) scrollOffset() int {
	visibleHeight := m.height*7/10 - 6
	if visibleHeight < 1 {
		visibleHeight = 10
	}

	if m.cursor < visibleHeight {
		return 0
	}
	return m.cursor - visibleHeight + 1
}
