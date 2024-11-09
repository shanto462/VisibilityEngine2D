using System.Runtime.CompilerServices;
using System.Windows;
using System.Windows.Media;
using System.Windows.Shapes;
using VisibilityIn2DGrid.Extensions;

namespace VisibilityIn2DGrid.RayTracing;

public class ShadowCast2D(Point origin, double radius, double step = 0.1f) : IDisposable
{
    private record Ray(Point origin, double angle, double distance)
    {
        public Point Origin { get; set; } = origin;
        public Vector Direction { get; set; } = new Vector(Math.Cos(angle), Math.Sin(angle));
        public double Distance { get; set; } = distance;
        public double Angle { get; set; } = angle;
    }

    private record RayHit(Point hitPoint, double distance, double angle)
    {
        public Point HitPoint { get; set; } = hitPoint;
        public double Distance { get; set; } = distance;
        public double Angle { get; set; } = angle;
    }

    private record LineSegment(Point start, Point end)
    {
        public Point Start { get; set; } = start;
        public Point End { get; set; } = end;
    }


    private readonly Point _origin = origin;
    private readonly double _radius = radius;

    private readonly List<LineSegment> _segments = [];
    private readonly HashSet<Polygon> _polygons = [];

    private readonly double _step = step;

    private bool disposedValue;

    private int RayCount => (int)(360 / _step);

    public void AddPolygon(Polygon polygon)
    {
        if (polygon.Points.Count < 3)
        {
            return;
        }

        _ = _polygons.Add(polygon);

        for (int i = 0; i < polygon.Points.Count; i++)
        {
            Point start = polygon.Points[i];
            Point end = polygon.Points[(i + 1) % polygon.Points.Count];

            _segments.Add(new LineSegment(start, end));
        }
    }

    public Polygon ComputeVisibility()
    {
        if (IsPointInAnyPolygon(_origin))
        {
            return new Polygon
            {
                Points = [],
                Fill = new SolidColorBrush(Color.FromArgb(0, 255, 255, 0)),
                Stroke = new SolidColorBrush(Colors.Transparent),
                StrokeThickness = 0
            };
        }

        List<RayHit> rayHits = [];

        _ = Parallel.For(0, RayCount, (i) =>
        {
            double angle = i * 2 * Math.PI / RayCount;
            Ray ray = new(_origin, angle, _radius);

            RayHit hit = CastRay(ray);

            if (hit is not null)
            {
                rayHits.Add(hit);
            }
        });

        rayHits = [.. rayHits.OrderBy(h => h.Angle)];

        PointCollection points = [];

        foreach (RayHit hit in rayHits)
        {
            points.Add(hit.HitPoint);
        }

        return new Polygon
        {
            Points = points,
            Fill = new SolidColorBrush(Color.FromArgb(90, 255, 255, 0)),
            Stroke = new SolidColorBrush(Colors.Yellow),
            StrokeThickness = 2
        };
    }

    private bool IsPointInAnyPolygon(Point point)
    {
        foreach (Polygon polygon in _polygons)
        {
            if (polygon.ContainsPoint(point))
            {
                return true;
            }
        }
        return false;
    }

    private RayHit CastRay(Ray ray)
    {
        RayHit? closestHit = null;
        double closestDistance = double.MaxValue;

        foreach (LineSegment segment in _segments)
        {
            if (TryGetIntersection(ray, segment, out Point intersection))
            {
                double distance = CalculateDistance(_origin, intersection);
                if (distance < closestDistance)
                {
                    closestDistance = distance;
                    closestHit = new RayHit(intersection, distance, ray.Angle);
                }
            }
        }

        if (closestHit == null)
        {
            Point endPoint = new(
                _origin.X + (ray.Direction.X * ray.Distance),
                _origin.Y + (ray.Direction.Y * ray.Distance)
            );
            closestHit = new RayHit(endPoint, ray.Distance, ray.Angle);
        }

        return closestHit;
    }

    private static bool TryGetIntersection(Ray ray, LineSegment segment, out Point intersection)
    {
        intersection = new Point();

        Point p3 = ray.Origin;
        Point p4 = new(
            ray.Origin.X + (ray.Direction.X * ray.Distance),
            ray.Origin.Y + (ray.Direction.Y * ray.Distance)
        );

        Point p1 = segment.Start;
        Point p2 = segment.End;

        double denominator = ((p1.X - p2.X) * (p3.Y - p4.Y)) - ((p1.Y - p2.Y) * (p3.X - p4.X));

        if (Math.Abs(denominator) < 0.000001)
        {
            return false;
        }

        double t = (((p1.X - p3.X) * (p3.Y - p4.Y)) - ((p1.Y - p3.Y) * (p3.X - p4.X))) / denominator;
        double u = -(((p1.X - p2.X) * (p1.Y - p3.Y)) - ((p1.Y - p2.Y) * (p1.X - p3.X))) / denominator;

        if (t >= 0 && t <= 1 && u >= 0 && u <= 1)
        {
            intersection.X = p1.X + (t * (p2.X - p1.X));
            intersection.Y = p1.Y + (t * (p2.Y - p1.Y));
            return true;
        }

        return false;
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static double CalculateDistance(Point p1, Point p2)
    {
        double dx = p2.X - p1.X;
        double dy = p2.Y - p1.Y;
        return Math.Sqrt((dx * dx) + (dy * dy));
    }

    private void Clear()
    {
        _segments.Clear();
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
