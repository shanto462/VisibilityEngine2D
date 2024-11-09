using System.Runtime.CompilerServices;
using System.Windows;
using System.Windows.Media;
using System.Windows.Shapes;
using VisibilityIn2DGrid.Extensions;
using VisibilityIn2DGrid.Geometry;

namespace VisibilityIn2DGrid.Index;

public class SpatialIndex(float width, float height, float gridSize) : IDisposable
{
    private readonly int columns = (int)Math.Ceiling(width / gridSize);
    private readonly int rows = (int)Math.Ceiling(height / gridSize);

    private readonly Dictionary<string, GridCell> grid = [];

    private bool disposedValue;

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static string GetCellKey(int column, int row)
    {
        return $"{column},{row}";
    }

    private List<GridCell> GetIntersectingCells(Polygon polygon)
    {
        Bounds bounds = polygon.GetBounds();

        int startCol = Math.Max(0, (int)(bounds.Min.X / gridSize));
        int endCol = Math.Min(columns - 1, (int)(bounds.Max.X / gridSize));
        int startRow = Math.Max(0, (int)(bounds.Min.Y / gridSize));
        int endRow = Math.Min(rows - 1, (int)(bounds.Max.Y / gridSize));

        List<GridCell> intersectingCells = [];

        for (int row = startRow; row <= endRow; row++)
        {
            for (int col = startCol; col <= endCol; col++)
            {
                if (PolygonIntersectsCell(polygon, col, row))
                {
                    string key = GetCellKey(col, row);

                    if (!grid.TryGetValue(key, out GridCell? value))
                    {
                        value = new GridCell(col, row, gridSize);
                        grid[key] = value;
                    }
                    intersectingCells.Add(value);
                }
            }
        }

        return intersectingCells;
    }

    private bool PolygonIntersectsCell(Polygon polygon, int col, int row)
    {
        float cellX = col * gridSize;
        float cellY = row * gridSize;

        Bounds bounds = polygon.GetBounds();

        return !(bounds.Max.X < cellX || bounds.Min.X > cellX + gridSize ||
                bounds.Max.Y < cellY || bounds.Min.Y > cellY + gridSize);
    }

    public void Insert(Polygon polygon)
    {
        List<GridCell> intersectingCells = GetIntersectingCells(polygon);

        foreach (GridCell cell in intersectingCells)
        {
            _ = cell.Polygon.Add(polygon);
        }
    }

    public IList<Polygon> QueryBounds(Point center, float visibilityRange)
    {
        Circle visibilityCircle = new(center, visibilityRange);

        Bounds circleBounds = visibilityCircle.GetBounds();

        int startCol = Math.Max(0, (int)(circleBounds.Min.X / gridSize));
        int endCol = Math.Min(columns - 1, (int)(circleBounds.Max.X / gridSize));
        int startRow = Math.Max(0, (int)(circleBounds.Min.Y / gridSize));
        int endRow = Math.Min(rows - 1, (int)(circleBounds.Max.Y / gridSize));

        HashSet<Polygon> result = [];

        for (int row = startRow; row <= endRow; row++)
        {
            for (int col = startCol; col <= endCol; col++)
            {
                string key = GetCellKey(col, row);

                if (grid.TryGetValue(key, out GridCell? cell))
                {
                    foreach (Polygon polygon in cell.Polygon)
                    {
                        if (visibilityCircle.IntersectsPolygon(polygon))
                        {
                            _ = result.Add(polygon);
                        }
                    }
                }
            }
        }
        return [.. result];
    }

    public (IList<Polygon> visiblePolygons, Polygon fovPolygon) QueryFOV(
        Point center,
        float fovAngle,
        float direction,
        float visibilityRange,
        float step = 0.1f
    )
    {
        IList<Polygon> potentialPolygons = QueryBounds(center, visibilityRange);

        double startAngle = (direction - (fovAngle / 2)) * Math.PI / 180;
        double endAngle = (direction + (fovAngle / 2)) * Math.PI / 180;

        Point leftPoint = new(
            center.X + (visibilityRange * Math.Cos(startAngle)),
            center.Y + (visibilityRange * Math.Sin(startAngle))
        );

        Point rightPoint = new(
            center.X + (visibilityRange * Math.Cos(endAngle)),
            center.Y + (visibilityRange * Math.Sin(endAngle))
        );

        Polygon fovPolygon = new();
        fovPolygon.Points.Add(new Point(center.X, center.Y));
        fovPolygon.Points.Add(leftPoint);

        int arcPoints = (int)(Math.Abs(endAngle - startAngle) / step);

        for (int i = 1; i < arcPoints; i++)
        {
            float t = i / (float)arcPoints;
            double angle = startAngle + ((endAngle - startAngle) * t);
            Point point = new(
                center.X + (visibilityRange * Math.Cos(angle)),
                center.Y + (visibilityRange * Math.Sin(angle))
            );
            fovPolygon.Points.Add(point);
        }

        fovPolygon.Points.Add(rightPoint);

        HashSet<Polygon> result = [];

        foreach (Polygon polygon in potentialPolygons)
        {
            if (PolygonsIntersect(fovPolygon, polygon))
            {
                _ = result.Add(polygon);
            }
        }

        return ([.. result], fovPolygon);
    }

    private static bool PolygonsIntersect(Polygon fov, Polygon target)
    {
        return IsAnyPointInPolygon(fov, target) || IsAnyPointInPolygon(target, fov) || DoEdgesIntersect(fov, target);
    }

    private static bool IsAnyPointInPolygon(Polygon container, Polygon points)
    {
        foreach (Point point in points.Points)
        {
            if (IsPointInPolygon(point, container.Points))
            {
                return true;
            }
        }
        return false;
    }

    private static bool IsPointInPolygon(Point point, PointCollection polygonPoints)
    {
        bool inside = false;
        int j = polygonPoints.Count - 1;

        for (int i = 0; i < polygonPoints.Count; j = i++)
        {
            if (((polygonPoints[i].Y > point.Y) != (polygonPoints[j].Y > point.Y)) &&
                (point.X < ((polygonPoints[j].X - polygonPoints[i].X) * (point.Y - polygonPoints[i].Y) /
                (polygonPoints[j].Y - polygonPoints[i].Y)) + polygonPoints[i].X))
            {
                inside = !inside;
            }
        }

        return inside;
    }

    private static bool DoEdgesIntersect(Polygon poly1, Polygon poly2)
    {
        for (int i = 0, j = poly1.Points.Count - 1; i < poly1.Points.Count; j = i++)
        {
            Point line1Start = poly1.Points[j];
            Point line1End = poly1.Points[i];

            for (int k = 0, l = poly2.Points.Count - 1; k < poly2.Points.Count; l = k++)
            {
                Point line2Start = poly2.Points[l];
                Point line2End = poly2.Points[k];

                if (DoLinesIntersect(line1Start, line1End, line2Start, line2End))
                {
                    return true;
                }
            }
        }
        return false;
    }

    private static bool DoLinesIntersect(Point p1, Point p2, Point p3, Point p4)
    {
        double denominator = ((p4.Y - p3.Y) * (p2.X - p1.X)) - ((p4.X - p3.X) * (p2.Y - p1.Y));

        if (denominator == 0)
        {
            return false;
        }

        double ua = (((p4.X - p3.X) * (p1.Y - p3.Y)) - ((p4.Y - p3.Y) * (p1.X - p3.X))) / denominator;
        double ub = (((p2.X - p1.X) * (p1.Y - p3.Y)) - ((p2.Y - p1.Y) * (p1.X - p3.X))) / denominator;

        return ua >= 0 && ua <= 1 && ub >= 0 && ub <= 1;
    }

    private void Clear()
    {
        grid.Clear();
    }

    protected virtual void Dispose(bool disposing)
    {
        if (!disposedValue)
        {
            if (disposing)
            {
                Clear();
            }

            disposedValue = true;
        }
    }

    public void Dispose()
    {
        Dispose(disposing: true);
        GC.SuppressFinalize(this);
    }
}
