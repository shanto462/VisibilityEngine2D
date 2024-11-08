using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Shapes;

namespace VisibilityIn2DGrid.Helper;

public static class GridLineGenerator
{
    private static readonly List<Line> gridLines = [];

    public static void DrawMajorGridLines(Canvas canvas, double width, double height, double GridSize, double GridLineThickness)
    {
        if (gridLines.Count != 0)
        {
            return;
        }

        for (double x = 0; x <= width; x += GridSize)
        {
            var line = new Line
            {
                X1 = x,
                Y1 = 0,
                X2 = x,
                Y2 = height,
                Stroke = Brushes.Gray,
                StrokeThickness = GridLineThickness * 2
            };
            gridLines.Add(line);
        }

        for (double y = 0; y <= height; y += GridSize)
        {
            var line = new Line
            {
                X1 = 0,
                Y1 = y,
                X2 = width,
                Y2 = y,
                Stroke = Brushes.Gray,
                StrokeThickness = GridLineThickness * 2
            };
            gridLines.Add(line);
        }

        AddToCanvas(canvas);
    }

    private static void AddToCanvas(Canvas canvas)
    {
        foreach (var line in gridLines)
        {
            canvas.Children.Add(line);
        }
    }
}
