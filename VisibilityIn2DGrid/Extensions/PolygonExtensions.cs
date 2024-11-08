using System.Numerics;
using System.Windows;
using System.Windows.Shapes;
using VisibilityIn2DGrid.Geometry;

namespace VisibilityIn2DGrid.Extensions;

public static class PolygonExtensions
{
    public static Bounds GetBounds(this Polygon polygon)
    {
        var minX = polygon.Points.Min(p => p.X);
        var minY = polygon.Points.Min(p => p.Y);
        var maxX = polygon.Points.Max(p => p.X);
        var maxY = polygon.Points.Max(p => p.Y);

        return new Bounds(new Vector2((float)minX, (float)minY), new Vector2((float)maxX, (float)maxY));
    }

    public static bool ContainsPoint(this Polygon polygon, Point point)
    {
        var points = polygon.Points;

        bool inside = false;
        for (int i = 0, j = points.Count - 1; i < points.Count; j = i++)
        {
            if (points[i].Y > point.Y != points[j].Y > point.Y &&
                point.X < (points[j].X - points[i].X) * (point.Y - points[i].Y) /
                (points[j].Y - points[i].Y) + points[i].X)
            {
                inside = !inside;
            }
        }
        return inside;
    }
}