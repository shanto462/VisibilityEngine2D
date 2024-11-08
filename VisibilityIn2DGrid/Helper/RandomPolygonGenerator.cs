using System.Windows;
using System.Windows.Media;
using System.Windows.Shapes;

namespace VisibilityIn2DGrid.Helper;

public static class RandomPolygonGenerator
{
    private static readonly Color[] colors =
    [
        Colors.CornflowerBlue,
        Colors.LightCoral,
        Colors.MediumAquamarine,
        Colors.LightGoldenrodYellow,
        Colors.PaleVioletRed,
        Colors.MediumPurple,
        Colors.LightSeaGreen,
        Colors.PeachPuff,
        Colors.IndianRed,
        Colors.MediumTurquoise
    ];

    public static Polygon GenerateRandomPolygon(double CanvasWidth, double CanvasHeight)
    {
        int pointCount = Random.Shared.Next(3, 9);

        double centerX = Random.Shared.Next(0, (int)CanvasWidth);
        double centerY = Random.Shared.Next(0, (int)CanvasHeight);

        double radius = Random.Shared.Next(5, 60);

        var points = new List<Point>();

        for (int i = 0; i < pointCount; i++)
        {
            double angle = (Math.PI * 2 * i) / pointCount;

            double pointRadius = radius * (0.5 + Random.Shared.NextDouble());

            double x = centerX + (Math.Cos(angle) * pointRadius);
            double y = centerY + (Math.Sin(angle) * pointRadius);

            points.Add(new Point(x, y));
        }

        var polygon = new Polygon
        {
            Points = new PointCollection(points),
            Fill = GetRandomBrush(),
            Stroke = Brushes.Black,
            StrokeThickness = 1,
            Opacity = 0.7,
            ToolTip = $"Center: ({centerX:F0}, {centerY:F0})\nPoints: {pointCount}"
        };

        polygon.MouseEnter += (s, e) =>
        {
            polygon.Opacity = 1.0;
            polygon.StrokeThickness = 2;
        };

        polygon.MouseLeave += (s, e) =>
        {
            polygon.Opacity = 0.7;
            polygon.StrokeThickness = 1;
        };

        return polygon;
    }

    private readonly static Dictionary<Color, SolidColorBrush> colorToBrush = [];

    private static SolidColorBrush GetRandomBrush()
    {
        var color = GetRandomColor();

        if (colorToBrush.TryGetValue(color, out SolidColorBrush? value))
        {
            return value;
        }
        else
        {
            var brush = new SolidColorBrush(color);
            colorToBrush.Add(color, brush);
            return brush;
        }
    }

    public static Color GetRandomColor()
    {
        return colors[Random.Shared.Next(colors.Length)];
    }
}
